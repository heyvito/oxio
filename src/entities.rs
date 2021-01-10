use crate::fs;
use std::collections::HashSet;
use crate::result::Operation;

#[derive(Debug, Clone)]
pub struct Item {
    pub group: String,
    pub name: String,
    pub value: String,
    pub filename: String,
}

impl Item {
    pub fn fill_value(self: &mut Item) -> Operation {
        let path = fs::cache_path().join(&self.filename);
        let item = fs::read_item(&path)?;
        self.value = item.value;
        Ok(())
    }

    pub fn delete(self: &mut Item) -> Operation {
        let path = fs::cache_path().join(&self.filename);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}

pub fn group_items(items: Vec<Item>) -> Vec<(String, Vec<Item>)> {
    let mut groups = HashSet::new();
    for item in &items {
        groups.insert(&item.group);
    }

    let mut result = Vec::with_capacity(groups.len());
    let mut groups = groups.iter().to_owned().collect::<Vec<_>>();
    groups.sort();

    for &group in groups {
        let mut items = items
            .iter()
            .to_owned()
            .filter(|&i| i.group.eq(&group.to_owned()))
            .collect::<Vec<_>>();
        items.sort_by_key(|i| &i.name);
        result.push((group.to_owned(), items
            .to_owned()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()));
    }

    result
}
