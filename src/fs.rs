use std::path::{Path, PathBuf};
use std::env;
use std::io::{ErrorKind, BufReader, BufRead, Write};
use std::fs::{File, OpenOptions};

use sha1::{Sha1, Digest};

use crate::entities;
use crate::entities::Item;
use crate::levenshtein;
use crate::result::{Operation, Result, Error, OxResult, OxError};

pub(crate) fn cache_path() -> PathBuf {
    Path::new(&format!("{}/.oxio.cache", env::var("HOME").unwrap())).to_path_buf()
}

fn ensure_cache(path: &PathBuf) -> Operation {
    if path.exists() {
        return if path.is_dir() {
            Ok(())
        } else {
            Err(Error::new(&format!("{} already exists and is not a directory.",
                                    path.to_str().unwrap())))
        };
    }
    std::fs::create_dir(path).into_ox_result()
}

fn read_component(buf: &mut BufReader<File>, path: &str) -> std::io::Result<String> {
    let mut bytes: Vec<u8> = Vec::new();
    if buf.read_until(0x00u8, &mut bytes)? == 0 {
        return Err(std::io::Error::new(ErrorKind::UnexpectedEof,
                                       format!("Invalid or corrupt entry at {}", path)));
    }
    if bytes.ends_with(&[0x00u8]) {
        bytes.pop();
    }
    Ok(String::from_utf8(bytes)
        .unwrap_or_else(|_| panic!("Invalid UTF8 value at {}", path)))
}

pub(crate) fn read_item(path: &PathBuf) -> Result<Item> {
    let mut buf = BufReader::new(File::open(path)?);
    let path_str = path.to_str().unwrap();
    let group = read_component(&mut buf, path_str)?;
    let name = read_component(&mut buf, path_str)?;
    let value = read_component(&mut buf, path_str)?;
    let filename = path.file_name().unwrap().to_str().unwrap().to_string();
    Ok(entities::Item { group, name, value, filename })
}

fn read_items(items: &[PathBuf]) -> Result<Vec<Item>> {
    let mut ret = Vec::with_capacity(items.len());
    for item in items {
        ret.push(read_item(item)?)
    }
    Ok(ret)
}

pub fn index() -> Result<usize> {
    let path = cache_path();
    ensure_cache(&path)?;

    let files = std::fs::read_dir(&path)?
        .filter(|i| i.is_ok())
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_ok() && e.file_type().unwrap().is_file())
        .filter(|f| f.file_name().to_str().unwrap().ne(".index"))
        .filter(|f| f.file_name().to_str().unwrap().ne(".gitignore"))
        .map(|f| f.path())
        .collect::<Vec<_>>();

    let items = read_items(&files)?;
    let mut index_size = 0;
    for item in &items {
        index_size += item.name.len() + item.group.len() + item.filename.len() + 3;
    }
    let mut buf: Vec<u8> = Vec::with_capacity(index_size);
    for item in &items {
        buf.append(&mut item.group.clone().into_bytes());
        buf.push(0x00u8);
        buf.append(&mut item.name.clone().into_bytes());
        buf.push(0x00u8);
        buf.append(&mut item.filename.clone().into_bytes());
        buf.push(0x00u8)
    }

    OpenOptions::new()
        .read(false)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path.join(".index"))?
        .write_all(buf.as_slice())?;

    Ok(items.len())
}

pub fn get_all_items() -> Result<Vec<Item>> {
    let index_path = cache_path().join(".index");
    if !index_path.exists() {
        return Ok(vec![]);
    }
    let index_path_str = index_path.to_str().unwrap();
    let mut buf = BufReader::new(File::open(&index_path)?);
    let mut items = Vec::new();
    loop {
        let group = match read_component(&mut buf, &index_path_str) {
            Ok(v) => v,
            Err(e) => if e.kind() == ErrorKind::UnexpectedEof {
                break;
            } else {
                return Err(e.as_ox_error());
            },
        };
        let name = read_component(&mut buf, &index_path_str)?;
        let filename = read_component(&mut buf, &index_path_str)?;
        items.push(Item { group, name, filename, value: "".to_string() })
    }
    Ok(items)
}

pub fn find_item(name: &str) -> Result<Option<Item>> {
    let mut items: Vec<(usize, Item)> = get_all_items()?
        .into_iter()
        .map(|i| (levenshtein::distance(&i.name, name), i))
        .collect();

    items.sort_by(|(a, _), (b, _)| a.cmp(b));
    if items.is_empty() {
        return Ok(None);
    }

    let (distance, first_item) = &items[0];
    if *distance > 2 {
        Ok(None)
    } else {
        Ok(Some(first_item.clone()))
    }
}

pub fn get_item(group: &str, name: &str) -> Result<Option<Item>> {
    Ok(get_all_items()?
        .into_iter()
        .find(|i| i.name.eq(name) && i.group.eq(group)))
}

pub fn create_item(group: &str, name: &str, value: &mut String) -> Operation {
    let cache_path = cache_path();
    ensure_cache(&cache_path)?;

    // Ensure index is updated
    index()?;

    // Delete if exists
    if let Some(mut it) = get_item(group, name)? {
        it.delete()?;
    }

    let name = name.to_lowercase();
    let mut buf = Vec::with_capacity(group.len() + name.len() + value.len() + 2);
    buf.append(group.as_bytes().to_vec().as_mut());
    buf.push(0x00u8);
    buf.append(name.as_bytes().to_vec().as_mut());
    buf.push(0x00u8);
    buf.append(value.as_bytes().to_vec().as_mut());
    let bytes = buf.as_slice();

    let mut hasher = Sha1::new();
    hasher.update(bytes.as_ref());

    let hash_digest = format!("{:x}", hasher.finalize());
    OpenOptions::new()
        .read(false)
        .write(true)
        .create(true)
        .truncate(true)
        .open(cache_path.join(hash_digest))?
        .write_all(bytes)?;
    // Reindex
    index()?;
    Ok(())
}

pub fn get_all_group(group: &str) -> Result<Vec<Item>> {
    Ok(get_all_items()?
        .into_iter()
        .filter(|i| i.group.eq(group))
        .collect::<Vec<_>>())
}

pub fn is_valid_name(name: &str) -> bool {
    let reserved_names = ["all", "rm-group", "rm-item", "sync", "reindex", "help", "edit"];
    !reserved_names.contains(&name)
}
