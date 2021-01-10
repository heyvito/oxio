# Oxio
[![crates.io](https://img.shields.io/crates/v/oxio.svg)](https://crates.io/crates/oxio)

Oxio is a small snippet manager inspired by [Zach Holman](https://github.com/holman/)'s [boom](https://github.com/holman/boom). Quoting Zach's description of boom:

> You can stash away text like URLs, canned responses, and important notes and then quickly copy them onto your clipboard, ready for pasting.

<center><img src="https://user-images.githubusercontent.com/77198/104183876-631cfd00-53f1-11eb-81a3-df01d302074c.png" /></center>

## Installing

Install using [`cargo`](https://crates.io/):



```
cargo install oxio
```

## Usage

```
 ▲ oxio gif magic http://i.imgur.com/n5xR79B.gif
oxio: Ok, magic (in gif) is http://i.imgur.com/n5xR79B.gif

 ▲ oxio magic
oxio: http://i.imgur.com/n5xR79B.gif (from gif->magic) is now in your clipboard!

 ▲ oxio rm-item gif magic
oxio: Removed magic from gif
```

### Manipulating Items

To add items to your local cache, invoke `oxio` passing three arguments;
A `group`, the item `name` and the item's `value`. Those arguments are used
to write a new file in the local cache:

```
▲ oxio gif magic http://i.imgur.com/n5xR79B.gif
  ------^----^-----------------^----------------
        |    |                 |
        |    |                 Value
        |    Name
        Group
```

To quickly copy an item to the clipboard, simply invoke `oxio` passing the
item name in the first argument:

```
▲ oxio magoc
oxio: http://i.imgur.com/n5xR79B.gif (from gif->magic) is now in your clipboard!
```

It then copies the item to your clipboard. Notice that the command above asked
for `magoc` instead of `magic`. Oxio attempts automatically fix typos.
In case you need an exact item, from an exact group, invoke `oxio` passing
the group, followed by the item's name:

```
▲ oxio gif magic
oxio: http://i.imgur.com/n5xR79B.gif (from gif->magic) is now in your clipboard!
```

To remove an item, use `rm-item`, again, passing the group's and item's name:

```
▲ oxio rm-item gif magic
oxio: Removed magic from gif
```

To remove a group and all its items, use `rm-group`:

```
▲ oxio rm-group gif magic
oxio: Removed group gif and all its items.
```

Then, to display all your items, use `oxio all`:

```
▲ oxio all
gif:
  magic: http://i.imgur.com/n5xR79B.gif
```

Alternatively, to edit large items using your default editor, use `oxio edit`,
passing the group's and item's name:

```
▲ oxio edit gif magic
[opens your editor]
oxio: Ok, magic (in gif) is foobar
```

### Sync

For those using multiple machines, Oxio is able to sync a repository
with all local items.
To start using sync, create a new remote repository, and use one of the
following alternatives:

#### With an existing cache
In case you already have a local cache (you already added items),
use `oxio sync merge git-url`, where `git-url` is a git repository:

```
▲ oxio sync merge git@github.com:yourusername/.oxio.git
oxio: Clonning git@github.com:yourusername/.oxio.git into /var/folders/l1/n7yv6s4d5350p7nxyn8bmtsc0000gn/T/zcl7bKG1cY0mvMcpG5Y9AGjpnICvVa
oxio: Copying items to new temporary repository...
oxio: Performing sync...
oxio: Merging changes...
oxio: Pushing changes...
oxio: Sync complete
oxio: Applying local changes...
oxio: Done! 220 item(s) in the local repository. Use oxio sync to sync changes.
```

Oxio will automatically update the repository and your installation.

#### Without an existing cache
In case you haven't added items into your local cache, or you already have
a repository with Oxio items and want to download to your machine, use
`oxio sync init git-url`:

```
▲ oxio sync init git@github.com:yourusername/.oxio.git
oxio: Clonning git@github.com:yourusername/.oxio.git into /Users/yourusername/.oxio.cache
oxio: Done! 219 item(s) in the local repository. Use oxio sync to sync changes.
```

#### Syncing changes to your local cache
After adding, removing or editing items or groups, simply invoke `oxio sync`.
Remote changes will be downloaded to the local repository, and local changes
will be sent to it.

```
▲ oxio sync
oxio: Performing sync...
oxio: Merging changes...
oxio: Pushing changes...
oxio: Sync complete
```

## TODO

- [ ] Add Tests

## License

```
MIT License

Copyright (c) 2021 Victor Gama

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
