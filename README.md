# rujutsu

A standalone [Magit](https://magit.vc/)-like TUI for [Jujutsu (jj)](https://github.com/jj-vcs/jj).

`rujutsu` gives you a keyboard-driven, buffer-oriented terminal interface for
jj: browse the log, inspect revisions and diffs, and drive common operations
(describe, commit, new, squash, rebase, bookmarks, push/fetch, undo/redo) from
transient menus — without leaving the terminal.

## Features

- **Status buffer** — working-copy summary and a configurable log section at a
  glance.
- **Log & revision buffers** — navigate the graph, expand sections, and open
  revisions to view their header and per-file diffs.
- **Transient menus** — Magit-style popups for describe, commit, new, squash,
  rebase, bookmark, push, fetch, and log.
- **Operation log** — browse `jj op log` and restore to a previous operation.
- **Evolog** — inspect a change's evolution.
- **Live refresh** — the repository is watched and buffers update as the
  working copy changes.
- **Copy to clipboard** — `y s` copies the value at point (a change id,
  operation id, or file path) and `y b` the buffer's revision; a real
  clipboard tool (wl-copy / xclip / xsel / pbcopy) is used when available so
  failures are reported, else OSC 52 (works over SSH).
- **Emacs-style key sequences** — multi-key bindings like `P p`, layered as
  transient > buffer-local > global.

## Requirements

- [`jj`](https://github.com/jj-vcs/jj) installed and on your `PATH`.
- A [Rust](https://www.rust-lang.org/) toolchain to build from source.

## Installation

```sh
cargo install --path .
```

Or build a release binary:

```sh
cargo build --release
# target/release/rujutsu
```

## Usage

Run inside a jj repository:

```sh
rujutsu
```

Press `?` for in-app help.

## Keybindings

Defaults (override any of these via config; see below):

### Navigation

| Key | Action |
| --- | --- |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `C-d` / `PgDn` | Half page down |
| `C-u` / `PgUp` | Half page up |
| `Home` | Go to top |
| `G` / `End` | Go to bottom |
| `n` / `p` (also `z j` / `z k`) | Next / previous section |
| `^` | Parent section |
| `TAB` (also `z a`) | Toggle (fold/unfold) section |
| `z A` | Toggle section recursively |
| `z o` / `z c` | Open / close section (`z c` again closes the parent) |
| `z O` / `z C` | Open / close section recursively |
| `z r` / `z m` | Open / close one fold level |
| `z R` / `z M` | Open / close all sections |
| `RET` / `d` | Visit item (open revision / diff) |
| `/` | Search in buffer |

### Actions

| Key | Action |
| --- | --- |
| `g` | Refresh |
| `x` | Abandon / restore |
| `e` | Edit |
| `a` | Absorb |
| `D` | Duplicate |
| `y s` / `y b` | Copy value at point / buffer revision to clipboard |
| `S` | Split |
| `v` | Evolog |
| `u` | Undo |
| `C-r` | Redo |
| `O` | Operation log |
| `$` | Process log |
| `q` | Quit |
| `?` | Help |

### Transient menus

| Key | Menu |
| --- | --- |
| `c` | Describe |
| `C` | Commit |
| `o` | New |
| `s` | Squash |
| `r` | Rebase |
| `b` | Bookmark |
| `P` | Push |
| `f` | Fetch |
| `l` | Log |

In the operation-log buffer, `r` restores to the selected operation instead of
opening the rebase menu.

## Configuration

`rujutsu` reads `$XDG_CONFIG_HOME/rujutsu/config.toml` (falling back to
`~/.config/rujutsu/config.toml`). Parse errors are reported as a startup
warning rather than aborting.

```toml
# Lines of context to keep around the cursor when scrolling.
scrolloff = 3

# Revset for the log section of the status buffer and the default log buffer.
# Omit to use jj's configured default (revsets.log).
# log_revset = "ancestors(@, 20)"

[keys.global]
"g"   = "refresh"
"P p" = "push"      # space-separated key sequences are supported

[keys.status]
"s" = "squash"

[keys.op-log]
"r" = "op-restore"

[colors]            # role names: see src/theme.rs
diff-add  = "green"
cursor-bg = "#3a3a3a"
key       = "42"    # 256-color index
```

Per-buffer keymap tables are `keys.global`, `keys.status`, `keys.log`,
`keys.revision`, and `keys.op-log`. User bindings are merged on top of the
built-in defaults.

## License

MIT
