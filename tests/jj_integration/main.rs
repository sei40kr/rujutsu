//! Integration tests that exercise the `jj` CLI adapter (`JjCli`) against a
//! real `jj` binary, one module per operation. Each test declares its own
//! repo with `common::Repo`, calls one `Jj` method with a typed operation,
//! and checks the repo through `Repo`'s own readers. A missing `jj` fails
//! the suite.
//!
//! The `*_in_editor` methods are not covered: they inherit the terminal.

mod common;

mod abandon;
mod absorb;
mod bookmark;
mod commit;
mod describe;
mod duplicate;
mod edit;
mod fetch;
mod log;
mod new;
mod op;
mod push;
mod rebase;
mod restore;
mod show;
mod squash;
mod status;
mod undo;
