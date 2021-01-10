use std::path::{PathBuf, Path};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

use git2::{Repository, ErrorCode, Signature, Config, RemoteCallbacks, Cred, Direction, PushOptions, IndexAddOption, ObjectType, Commit, FetchOptions, StatusOptions};
use colored::Colorize;

use crate::fs;
use crate::sync::CanSync::*;
use crate::result::{Result, Error, OxResult, Operation, OxError};
use crate::ox_println;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use crate::fs::cache_path;

pub enum CanSync {
    Yes,
    NotConfigured(String),
    NoLocalCache,
    NoRemotes,
}

pub fn can_sync_cache() -> Result<CanSync> {
    let cache_path = fs::cache_path();
    // Cache exists?
    if !cache_path.exists() {
        return Ok(NoLocalCache);
    }

    match Repository::open(cache_path) {
        Err(err) => Ok(NotConfigured(err.message().to_string())),
        Ok(rep) => {
            if rep.remotes()?.is_empty() {
                return Ok(NoRemotes);
            }
            Ok(Yes)
        }
    }
}

fn get_git_config() -> Result<Signature<'static>> {
    let conf = Config::open_default()?;
    let user = match conf.get_entry("user.name")?.value() {
        None => return Err(Error::new("Don't know who you are. Please configure git.")),
        Some(v) => v.to_string()
    };
    let email = match conf.get_entry("user.email")?.value() {
        None => return Err(Error::new("Don't know who you are. Please configure git.")),
        Some(v) => v.to_string()
    };
    Signature::now(user.as_str(), email.as_str()).into_ox_result()
}

fn ssh_callbacks() -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, allowed_types| {
        if allowed_types.is_username() {
            return Cred::username(username_from_url.unwrap());
        }

        if allowed_types.is_ssh_key() {
            Cred::ssh_key(
                username_from_url.unwrap(),
                None,
                std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
                None,
            )
        } else {
            Err(git2::Error::from_str("unable to get private key"))
        }
    });
    callbacks
}

fn clone(url: String, into: &PathBuf) -> Result<Repository> {
    ox_println!("Clonning {} into {}", url, into.to_str().unwrap());
    let callbacks = ssh_callbacks();
    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(callbacks);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);

    builder.clone(url.as_str(), into).into_ox_result()
}

fn push(repo: &Repository, ref_spec: &str) -> Operation {
    let mut remote = match repo.find_remote("origin") {
        Ok(r) => r,
        Err(_) => return Err(Error::new("No remote configured for local cache."))
    };

    remote.connect_auth(Direction::Push, Some(ssh_callbacks()), None)?;
    let mut push_opts = PushOptions::new();
    push_opts.remote_callbacks(ssh_callbacks());
    remote.push(&[ref_spec], Some(&mut push_opts)).into_ox_result()
}

fn prepare(repo: &Repository) -> Operation {
    let signature = get_git_config()?;
    // Ok, do we have a branch?
    let head = match repo.head() {
        Ok(h) => h.name().unwrap().to_string(),
        Err(ref e) if e.code() == ErrorCode::NotFound || e.code() == ErrorCode::UnbornBranch => {
            repo.set_head("refs/heads/main")?;
            "refs/heads/main".to_string()
        }
        Err(e) => return Err(e.as_ox_error()),
    };

    let gi = repo.path().parent().unwrap().join(".gitignore");
    let mut gi_added = false;
    if gi.exists() {
        // Check whether we're ignoring our .index
        let mut contents = String::new();
        if let Err(e) = File::open(&gi)
            .and_then(|mut f| f.read_to_string(&mut contents)) {
            return Err(e.as_ox_error());
        }
        let mut contents = contents.split('\n').collect::<Vec<_>>();
        if contents.contains(&".index") {
            return Ok(());
        }

        // Add index, update gitignore
        contents.push(".index");
        if let Err(e) = OpenOptions::new().write(true).truncate(true).open(&gi) {
            return Err(e.as_ox_error());
        }
    } else {
        if OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&gi)
            .and_then(|mut f| f.write_all(".index\n".as_bytes()))
            .is_err() {
            return Err(Error::new("Could not write .gitignore"));
        }
        gi_added = true;
    }

    let mut idx = repo.index()?;
    idx.add_path(Path::new(".gitignore"))?;
    let oid = idx.write_tree()?;
    idx.write()?;
    let tree = repo.find_tree(oid)?;
    let parents = get_parent_commit(&repo)?;
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        if gi_added {
            "Add .gitignore"
        } else {
            "Update .gitignore"
        },
        &tree,
        parents.iter().collect::<Vec<_>>().as_slice())?;
    push(&repo, head.as_str())
}

pub fn init_sync_empty(remote: String) -> Operation {
    let cache_path = fs::cache_path();
    if cache_path.exists() {
        return Err(Error::new("Cache storage already exists."));
    }

    let repo = clone(remote, &cache_path)?;
    prepare(&repo)?;
    let items = fs::index()?;
    ox_println!("Done! {} item(s) in the local repository. Use {} to sync changes.", format!("{}", items).magenta(), "oxio sync".yellow());
    Ok(())
}

fn get_parent_commit(repo: &Repository) -> Result<Vec<Commit>> {
    repo.head()
        .and_then(|h| h.resolve())
        .and_then(|r| r.peel(ObjectType::Commit))
        .and_then(|c| c.into_commit().map_err(|_| git2::Error::from_str("No commit available")))
        .map(|c| vec!(c)).into_ox_result()
        .or_else(|_| Ok(Vec::with_capacity(0)))
}

fn stage_current_changes(repo: &Repository) -> Result<git2::Oid> {
    let sig = get_git_config()?;
    let mut idx = repo.index()?;
    idx.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    let oid = idx.write_tree()?;
    idx.write()?;

    let tree = repo.find_tree(oid)?;
    let parents = get_parent_commit(&repo)?;

    let msg = "Update items";
    let up_ref = Some("HEAD");
    repo.commit(up_ref, &sig, &sig, msg, &tree, parents.iter().collect::<Vec<_>>().as_slice())?;

    Ok(oid)
}

pub fn perform_sync(repo: &Repository) -> Operation {
    ox_println!("Performing sync...");
    let sig = get_git_config()?;
    let mut stat_opts = StatusOptions::new();
    stat_opts.include_ignored(false);
    stat_opts.include_untracked(true);
    let statuses = repo.statuses(Some(&mut stat_opts))?;
    let should_push = if !statuses.is_empty() {
        stage_current_changes(repo)?;
        true
    } else {
        false
    };

    let current_branch = repo.head()
        .map(|r| r.name().map(|v| v.to_string()).or(None))?
        .ok_or_else(|| Error::new("Unexpected error: Detached HEAD?"))?;

    // Fetch data
    let remote_name = repo.remotes()
        .map(|arr| arr.get(0).map(|v| v.to_string()).or(None))?
        .ok_or_else(|| Error::new("Unexpected error: Remote does not have an origin."))?;
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(ssh_callbacks());

    repo.find_remote(remote_name.as_str())
        .map_err(|_| Error::new("Unexpected error: Found remote, but then it didn't exist."))
        .and_then(|mut r| r.fetch(
            &[current_branch.as_str()],
            Some(&mut fo),
            Some("Automatic fetch")).into_ox_result())?;
    ox_println!("Merging changes...");
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let current_head = repo.find_reference("HEAD")?;
    let current_commit = repo.reference_to_annotated_commit(&current_head)?;
    let mut rebase = repo.rebase(
        Some(&current_commit),
        Some(&fetch_commit),
        Some(&current_commit),
        None,
    )?;
    rebase.finish(Some(&sig))?;
    repo.set_head(&current_branch)?;
    let mut result = Ok(());
    if should_push {
        ox_println!("Pushing changes...");
        result = push(repo, current_branch.as_str());
    }
    ox_println!("Sync complete");
    result
}

pub fn init_sync_existing(remote: String) -> Operation {
    let path = fs::cache_path();
    if !path.exists() {
        return init_sync_empty(remote);
    }

    // Ok, let's check whether our store has eny items. In case it does not, we can safely
    // drop and replace it. :)

    // Refuse to operate on an existing repository.
    if Repository::open(&path).is_ok() {
        return Err(Error::new("Repository already initialized"));
    }

    // Ok, then let's try to reindex it and see how many items we have.
    fs::index()?;
    let current_items = fs::get_all_items()?;
    if current_items.is_empty() {
        // No items here. Let's drop the directory, clone, and be done with it.
        std::fs::remove_dir_all(&path)?;
        init_sync_empty(remote)?;
    } else {
        // Aight, in this case we have an existing folder, without a repo. Let's clone the repo aside,
        // migrate all items into it, push it, and replace the local copy with the brand new repo.
        // After that, we can reindex it and yay!
        let current_repo = fs::cache_path();
        let tmp_name: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        let tmp_repo = std::env::temp_dir().join(tmp_name);
        let repo = clone(remote, &tmp_repo)?;
        prepare(&repo)?;
        ox_println!("Copying items to new temporary repository...");
        // Copy items to the new repo
        for item in current_items {
            std::fs::copy(current_repo.join(&item.filename), tmp_repo.join(&item.filename))?;
        }

        // And sync
        perform_sync(&repo)?;

        ox_println!("Applying local changes...");
        // Then we replace the local copy with the new one.
        std::fs::remove_dir_all(&current_repo)?;
        std::fs::rename(tmp_repo, current_repo)?;
    }

    // And reindex our new instance.
    let items = fs::index()?;
    ox_println!("Done! {} item(s) in the local repository. Use {} to sync changes.", format!("{}", items).magenta(), "oxio sync".yellow());
    Ok(())
}

pub fn get_local_repository() -> Result<Repository> {
    Repository::open(cache_path()).into_ox_result()
}
