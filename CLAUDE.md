# rujutsu

A standalone Magit-like TUI for Jujutsu (jj), written in Rust (ratatui).
Modeled after [majutsu.el](https://github.com/0WD0/majutsu) — the Magit-style
jujutsu interface for Emacs — as its standalone (editor-independent) version.
Sibling project of [rugit](https://github.com/sei40kr/rugit) (the git
equivalent) — the two share the same architecture, and conventions established
there apply here.

## Commands

- `cargo build` — binary at `target/debug/rujutsu`
- `cargo test` — unit + integration tests. Integration tests
  (`tests/jj_integration.rs`) shell out to a real `jj` binary and skip
  silently when it is not installed.
- `cargo test <name>` — run a single test; `cargo test --test jj_integration`
  for just the integration suite.
- `cargo clippy --all-targets -- -D warnings` — CI enforces zero warnings.
- `cargo fmt --all --check` — CI enforces formatting.

The Nix devshell (`nix develop`, auto-entered via direnv) provides the Rust
toolchain plus Linux clipboard tools; its pre-commit hook runs treefmt / nil /
statix on Nix files.

## Architecture

Elm-style loop: `main.rs` owns the terminal and drains one
`crossbeam_channel<AppEvent>`; `App::update` mutates state; `ui/render.rs`
draws. Everything — terminal input, worker-thread completions, `.jj` fs
notifications — funnels into that single channel (`src/event.rs`).

- **jj via CLI, parsers pure** — `jj/client.rs` shells out to `jj`;
  `jj/parse.rs` is I/O-free and unit-tested against fixture strings. Log rows
  use a NUL-delimited template (`LOG_TEMPLATE`) so the graph prefix and fields
  split unambiguously.
- **Reads never snapshot** — all jj reads pass `--ignore-working-copy` so they
  don't write an operation and re-trigger the fs watcher. The one deliberate
  exception is the working-copy read at the top of `read_snapshot`, which
  snapshots so external edits show up.
- **Mutations on worker threads** — `app/workers.rs::run_jj_bg` spawns a
  thread; completion returns as `AppEvent::JjDone` and triggers a refresh.
  Generation counters (`refresh_gen`) plus `refresh_inflight`/`refresh_dirty`
  discard stale snapshots and coalesce watcher bursts.
- **Section trees** — every buffer (status, log, revision, op-log, process
  log) is a tree of `Section`s (`ui/section.rs`) flattened into `FlatLine`s.
  Builders live in `ui/build.rs`. Trees are rebuilt, never mutated; cursor,
  scroll, and fold state survive a refresh via `Pane::replace_tree`
  (identity-stable `SectionId`s).
- **Commands are data** — keymaps bind keys to `Command` enum values
  (`command.rs`), which makes TOML remapping and the help UI free.
  `keymap.rs` is a key-sequence trie with Emacs notation ("C-d", "P p");
  lookup layers transient > buffer-local > global.
- **Key routing priority** — `app/keys.rs`: confirm > minibuffer input > help
  > transient > keymap.
- **Transients are data** — `ui/transient.rs` defines menus as groups of
  switches/actions; `app/ops/` has one module per menu family (revise,
  rebase, bookmark, remote, log). Adding a menu: a `TransientDef` +
  `menu_def` arm in `ui/transient.rs`, a `Menu` variant in `command.rs`, a
  new file under `app/ops/`, and one routing arm in each of the two matches
  in `app/ops/mod.rs`. `App::dispatch` never grows.
- **DWIM dispatch** — `app/dwim.rs`: commands act on the `SectionValue` at
  point (e.g. `x` abandons a revision or restores a file depending on
  cursor position).
- **Terminal handoff** — `$EDITOR`-based commands (describe, commit, split)
  go through `EditorRequest`: the main loop leaves raw mode, pauses the input
  thread, runs jj in the foreground, then restores.

## Invariants

- No hardcoded colors in build/render code — every color is a named role on
  `Theme` (`src/theme/mod.rs`), overridable from `[colors]` in config. New
  theme presets are one file under `src/theme/` plus a `PRESETS` row.
- Pure modules stay I/O-free: `jj/parse.rs`, `ui/section.rs`, `ui/pane.rs`.
- `App::update` and `App::dispatch` are routers — behavior lives in the
  `app/` submodules (keys, dwim, search, workers, ops).
- Config parse errors surface as startup warnings, never aborts.

## Configuration

`$XDG_CONFIG_HOME/rujutsu/config.toml`. For the current fields and format,
see `src/config.rs` module docs and README — keep those two in sync when
changing config.
