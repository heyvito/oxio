use std::iter::Skip;
use std::env::Args;
use std::process::exit;
use std::iter;

use clipboard::ClipboardProvider;
use clipboard::ClipboardContext;
use atty::Stream;
use colored::Colorize;

use crate::HowNormalize::*;
use crate::entities::{Item, group_items};
use crate::sync::CanSync;

mod fs;
mod entities;
mod levenshtein;
mod sync;
mod result;
mod print;

fn help() {
    let help_str = format!(r"
NAME:
   {ox} - oxio is a simple kv storage inspired by Boom, originally
          written by Zach Holman.

HOMEPAGE:
   https://github.com/heyvito/oxio

AUTHOR:
    Victor 'Vito' Gama <hey@vito.io>


USAGE:
   {ox} {itemna}                    Finds and copies {itemna} to
                                    clipboard
   {ox} {grpname} {itemna}          Finds exactly {itemna} in {grpname}
   {ox} {grpname} {itemna} {val}    Sets {val} to {itemna} in {grpname}
   {ox} {ed} {grpname} {itemna}     Opens the default editor to edit or
                                    create {itemna} in {grpname}
   {ox} {l}                         Lists all items
   {ox} {rm_grp} {grpname}          Removes a group and all its items
   {ox} {rm_it} {grpname} {itemna}  Removes {itemna} from {grpname}
   {ox} {sn}                        Syncs all items and rebuilds the
                                    index. See README on how to use this
   {ox} {sn} {ni} {u}               Initializes the local cache with contents
                                    from the provided Git URL. After that, use
                                    {ox} {sn} to update the remote repository
                                    and local cache.
   {ox} {sn} {mrg} {u}              Merges the local cache with contents from
                                    the provided Git URL. After that, use
                                    {ox} {sn} to update the remote repository
                                    and local cache.
   {ox} {rindx}                     Forces all items in the local cache
                                    to be reindexed
   {ox} {hp}                        Shows this message

VERSION:
   0.1.3
", ox = "oxio".cyan(), itemna = "ITEMNAME".blue(), grpname = "GROUPNAME".blue(),
                           val = "VALUE".blue(), u = "URL".blue(),
                           l = "all".yellow(), rm_grp = "rm-group".yellow(), rm_it = "rm-item".yellow(),
                           sn = "sync".yellow(), ni = "init".yellow(), mrg = "merge".yellow(),
                           rindx = "reindex".yellow(), hp = "help".yellow(), ed = "edit".yellow());
    eprintln!("{}", help_str);
}

enum HowNormalize<'a> {
    AsIs(&'a mut Skip<Args>),
    Lowercase(&'a mut Skip<Args>),
}

fn truncate_output(s: &mut String) -> String {
    if s.contains('\n') {
        if s.len() <= 60 {
            s.to_string()
        } else {
            let mut new_str = s.replace("\n", " ");
            new_str.truncate(60);
            new_str + "..."
        }
    } else {
        s.to_string()
    }
}

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

fn normalize_argument(how: HowNormalize) -> String {
    match how {
        AsIs(s) => s.next().unwrap(),
        Lowercase(s) => s.next().unwrap().to_lowercase()
    }
}

fn largest_item_name(items: &[Item]) -> usize {
    if let Some(i) = items.iter().max_by(|a, b| a.name.len().cmp(&b.name.len())) {
        i.name.len()
    } else {
        0
    }
}

fn copy_or_echo(mut i: Item) {
    let res = i.fill_value();
    if res.is_err() {
        ox_eprintln!("Error loading item {}: {}", i.filename, res.err().unwrap());
        ox_eprintln!("Try running {}.", "oxio reindex".yellow());
        exit(1);
    }

    if atty::is(Stream::Stdout) {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        let res = ctx.set_contents(i.value.clone());
        if res.is_err() {
            ox_eprintln!("Error writing to clipboard: {}", res.err().unwrap());
            exit(1);
        }
        ox_println!("{} (from {}->{}) is now in your clipboard!", i.value.magenta(), i.group.blue(), i.name.blue());
    } else {
        println!("{}", i.value);
    }
}

// help, all, sync, reindex, <itm>

fn handle_one_word(args: &mut Skip<Args>) {
    match normalize_argument(Lowercase(args)).as_str() {
        "help" => help(),
        "all" => {
            match fs::get_all_items() {
                Err(err) => {
                    ox_eprintln!("Error reading items: {}", err);
                    exit(1)
                }
                Ok(items) => {
                    if items.is_empty() {
                        ox_eprintln!("Your store is empty. Use {} to create a new item", "oxio GROUP ITEM VALUE".yellow());
                        return;
                    }
                    let groups = group_items(items);
                    for (n, items) in groups {
                        println!("{}:", n.yellow());
                        let max_name = largest_item_name(&items);
                        for mut i in items {
                            if let Err(e) = i.fill_value() {
                                ox_eprintln!("Error loading item {}: {}", i.filename, e);
                                exit(1)
                            }
                            let pad = iter::repeat(" ").take(max_name - i.name.len()).collect::<String>();
                            println!("  {}{}: {}", pad, i.name.blue(), truncate_output(&mut i.value).magenta())
                        }
                        println!();
                    }
                }
            }
        }
        "sync" => {
            let status = match sync::can_sync_cache() {
                Err(e) => {
                    ox_eprintln!("Error determining repository status: {}", e);
                    exit(1)
                }
                Ok(status) => status
            };

            match status {
                CanSync::Yes => {
                    let repo = match sync::get_local_repository() {
                        Err(e) => {
                            ox_eprintln!("Error reading local repository: {}", e);
                            exit(1)
                        }
                        Ok(r) => r
                    };
                    let sync_result = match sync::perform_sync(&repo) {
                        Err(e) => {
                            ox_eprintln!("Error performing sync: {}", e);
                            exit(1);
                        }
                        Ok(()) => fs::index()
                    };
                    match sync_result {
                        Err(e) => {
                            ox_eprintln!("Error indexing local cache: {}", e);
                            exit(1);
                        }
                        Ok(items) => ox_println!("Sync completed. {} item(s) on local cache.", items)
                    }
                }
                CanSync::NotConfigured(reason) => {
                    ox_eprintln!("Cannot perform sync: {}", reason);
                    exit(1);
                }
                CanSync::NoRemotes => {
                    ox_eprintln!("Cannot perform sync: The cache already contains a repository, but it does not contain a remote.");
                    exit(1);
                }
                CanSync::NoLocalCache => {
                    ox_eprintln!("Cannot perform sync: You don't have a local cache. Either initialize a new by adding new items, or use {} to download a repository", "oxio sync init URL".yellow());
                    ox_eprintln!("For further information, please refer to the README.");
                    exit(1);
                }
            }
        }
        "reindex" => {
            match fs::index() {
                Err(e) => {
                    ox_eprintln!("Error reindexing: {}", e);
                    exit(1);
                }
                Ok(len) => ox_println!("Reindex completed. {} item(s)", len)
            }
        }
        name => {
            match fs::find_item(name) {
                Err(err) => {
                    ox_eprintln!("Error searching items: {}", err);
                    exit(1)
                }
                Ok(opt) => {
                    if let Some(item) = opt {
                        copy_or_echo(item)
                    } else {
                        ox_eprintln!("No item named {} was found", name.blue());
                        exit(1)
                    }
                }
            }
        }
    }
}

// rm-group, <grp> <itm>

fn handle_two_words(args: &mut Skip<Args>) {
    match normalize_argument(Lowercase(args)).as_str() {
        "rm-group" => {
            let group = normalize_argument(Lowercase(args));
            match fs::get_all_group(&group) {
                Err(e) => {
                    ox_eprintln!("Error loading items: {}", e);
                    exit(1)
                }
                Ok(items) => {
                    if items.is_empty() {
                        ox_eprintln!("No group named {} found.", group.yellow());
                        exit(1)
                    }
                    for mut i in items {
                        if let Err(e) = i.delete() {
                            ox_eprintln!("Error removing {}: {}", i.filename, e);
                            exit(1)
                        }
                    }
                    match fs::index() {
                        Err(e) => {
                            ox_eprintln!("Error reindexing: {}", e);
                            exit(1)
                        }
                        Ok(_) => ox_println!("Removed group {} and all its items.", group.yellow())
                    }
                }
            }
        }
        group_name => {
            let item_name = normalize_argument(Lowercase(args));
            match fs::get_item(&group_name.to_string(), &item_name) {
                Err(e) => {
                    ox_eprintln!("Error obtaining item: {}", e);
                    exit(1)
                }
                Ok(item) => {
                    if let Some(i) = item {
                        copy_or_echo(i);
                    } else {
                        ox_eprintln!("Could not find an item named {} on group {}", item_name.blue(), group_name.yellow());
                        exit(1)
                    }
                }
            }
        }
    }
}

// rm-item, edit, sync <>, add new item

fn handle_three_words(args: &mut Skip<Args>) {
    match normalize_argument(Lowercase(args)).as_str() {
        "rm-item" => {
            let group = normalize_argument(Lowercase(args));
            let name = normalize_argument(Lowercase(args));
            match fs::get_item(&group, &name) {
                Err(err) => {
                    ox_eprintln!("Error locating {}: {}", name.blue(), err);
                    exit(1);
                }
                Ok(item) => {
                    if let Some(mut item) = item {
                        if let Err(err) = item.delete() {
                            ox_eprintln!("Error removing {} from {}: {}", name.blue(), group.yellow(), err);
                            exit(1);
                        }
                        ox_println!("Removed {} from {}", name.blue(), group.yellow());
                    } else {
                        ox_eprintln!("Could not find {} in {}", name.blue(), group.yellow());
                        exit(1)
                    }
                }
            }
            if let Err(e) = fs::index() {
                ox_eprintln!("Error reindexing local cache: {}", e);
                exit(1)
            }
        }
        "sync" => handle_sync_command(normalize_argument(Lowercase(args)).as_str(), args),
        "edit" => handle_edit_command(args),
        group => {
            if !fs::is_valid_name(&group.to_string()) {
                ox_eprintln!("Invalid group name {}", group.yellow());
                exit(1)
            }

            let name = normalize_argument(Lowercase(args));
            if !fs::is_valid_name(&name) {
                ox_eprintln!("Invalid item name {}", name.blue());
                exit(1)
            }
            let mut value = normalize_argument(AsIs(args));
            match fs::create_item(&group.to_string(), &name, &mut value) {
                Err(e) => {
                    ox_eprintln!("Error creating item: {}", e);
                    exit(1);
                }
                Ok(()) => {
                    ox_println!("Ok, {} (in {}) is {}", name.blue(), group.yellow(), value.magenta());
                }
            }
        }
    }
}

// sync init, sync merge

fn handle_sync_command(cmd: &str, args: &mut Skip<Args>) {
    let url = normalize_argument(AsIs(args));
    match cmd {
        "init" => {
            if let Err(e) = sync::init_sync_empty(url) {
                ox_eprintln!("Error executing: {}", e);
                exit(1)
            }
        }
        "merge" => {
            if let Err(e) = sync::init_sync_existing(url) {
                ox_eprintln!("Error executing: {}", e);
                exit(1)
            }
        }
        _ => {
            ox_eprintln!("Unknown command 'sync {}'. Use {} for available options.", cmd, "oxio help".yellow());
            exit(1)
        }
    }
}

fn handle_edit_command(args: &mut Skip<Args>) {
    let group = normalize_argument(Lowercase(args));
    let item = normalize_argument(Lowercase(args));
    if !fs::is_valid_name(&group) {
        ox_eprintln!("Invalid group name {}", group.yellow());
        exit(1)
    }

    if !fs::is_valid_name(&item) {
        ox_eprintln!("Invalid item name {}", item.blue());
        exit(1)
    }

    let mut value = String::new();
    let entry = match fs::get_item(&group, &item) {
        Err(e) => {
            ox_eprintln!("Error searching index: {}", e);
            exit(1)
        }
        Ok(e) => e
    };

    if let Some(mut i) = entry {
        value = match i.fill_value() {
            Err(e) => {
                ox_eprintln!("Error reading item: {}", e);
                exit(1)
            }
            Ok(()) => i.value
        }
    }

    let mut edited = match edit::edit(&value) {
        Err(e) => {
            ox_eprintln!("There was a problem with your editor: {}", e);
            exit(1)
        }
        Ok(val) => val
    };
    trim_newline(&mut edited);

    match fs::create_item(&group, &item, &mut edited.to_string()) {
        Err(e) => {
            ox_eprintln!("Error writing item: {}", e);
            exit(1)
        }
        Ok(()) => {
            ox_println!("Ok, {} (in {}) is {}", item.blue(), group.yellow(), truncate_output(&mut edited).magenta());
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    match args.len() {
        1 => handle_one_word(&mut args),
        2 => handle_two_words(&mut args),
        3 => handle_three_words(&mut args),
        _ => help(),
    }
}
