//! One module per transient menu family, mirroring how Magit splits into
//! magit-commit.el / magit-branch.el / magit-push.el. Adding a menu means:
//! a `MenuDef` + `open` arm in `ui/transient.rs`, a `Menu` variant in
//! `command.rs`, a new file here, and one routing arm in `invoke_transient`
//! below. `App::dispatch` never grows.
//!
//! Each module completes a `TransientAction` into a `jj::op` value — adding
//! the revision at point, a picked bookmark, a typed name — and hands it to
//! the port. Nothing here spells a command line.

mod bookmark;
mod log;
mod rebase;
mod remote;
mod restore;
mod revise;

use crate::command::Menu;
use crate::ui::transient::{self, TransientAction};

use super::App;

impl App {
    pub(super) fn open_transient(&mut self, menu: Menu) {
        self.transient = Some(transient::open(menu));
    }

    /// Route a transient action to the module that owns its menu.
    pub(super) fn invoke_transient(&mut self, action: TransientAction) {
        use TransientAction::*;
        match action {
            Describe(..) | Commit(..) | New(..) | Squash(..) | SquashInto(..) => {
                self.revise_action(action)
            }
            Rebase(kind, emptied) => self.rebase_action(kind, emptied),
            BookmarkCreate | BookmarkSet | BookmarkDelete | BookmarkRename | BookmarkTrack
            | BookmarkUntrack | BookmarkForget => self.bookmark_action(action),
            Push(..) | PushBookmark(..) | PushChange(..) | Fetch(..) => self.remote_action(action),
            LogDefault(..) | LogAll(..) | LogRevset(..) => self.log_action(action),
            Restore(..) | RestoreChangesIn(..) => self.restore_action(action),
        }
    }
}
