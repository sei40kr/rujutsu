//! One module per transient menu family, mirroring how Magit splits into
//! magit-commit.el / magit-branch.el / magit-push.el. Adding a menu means:
//! a `TransientDef` + `menu_def` arm in `ui/transient.rs`, a `Menu` variant
//! in `command.rs`, a new file here, and one routing arm in each of the two
//! matches below. `App::dispatch` never grows.

mod bookmark;
mod log;
mod rebase;
mod remote;
mod revise;

use crate::command::Menu;
use crate::ui::transient::{menu_def, TransientAction, TransientState};

use super::App;

impl App {
    pub(super) fn open_transient(&mut self, menu: Menu) {
        self.transient = Some(TransientState::new(menu_def(menu)));
    }

    /// Route a transient action to the module that owns its menu.
    pub(super) fn invoke_transient(&mut self, action: TransientAction, args: Vec<String>) {
        use TransientAction::*;
        match action {
            Describe | Commit | NewChild | NewAfter | NewBefore | Squash | SquashInto => {
                self.revise_action(action, args)
            }
            RebaseRev | RebaseSource | RebaseBranch => self.rebase_action(action, args),
            BookmarkCreate | BookmarkSet | BookmarkDelete | BookmarkRename | BookmarkTrack
            | BookmarkUntrack | BookmarkForget => self.bookmark_action(action),
            Push | PushBookmark | PushChange | Fetch | FetchAllRemotes => {
                self.remote_action(action, args)
            }
            LogDefault | LogAll | LogRevset => self.log_action(action, args),
        }
    }
}
