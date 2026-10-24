//! Git-interop actions: fetch and push.

use crate::jj::op::{BookmarkName, BookmarkRef, Fetch, Push, PushScope};
use crate::ui::transient::TransientAction;

use super::super::{App, Input};

impl App {
    pub(super) fn remote_action(&mut self, action: TransientAction) {
        match action {
            TransientAction::Fetch(op) => {
                let desc = match op {
                    Fetch::DefaultRemote => "fetch",
                    Fetch::AllRemotes => "fetch all remotes",
                };
                self.run_bg(desc, move |jj| jj.fetch(&op));
            }
            TransientAction::Push(options) => {
                let op = Push {
                    scope: PushScope::Tracked,
                    options,
                };
                self.run_bg("push", move |jj| jj.push(&op));
            }
            TransientAction::PushBookmark(options) => {
                let candidates: Vec<String> = self
                    .snapshot
                    .as_ref()
                    .map(|s| {
                        s.bookmarks
                            .iter()
                            .filter(|b| matches!(b, BookmarkRef::Local(_)))
                            .map(ToString::to_string)
                            .collect()
                    })
                    .unwrap_or_default();
                if candidates.is_empty() {
                    self.message = Some("no local bookmarks".into());
                } else {
                    self.input = Some(Input::picker(
                        "Push bookmark",
                        candidates,
                        move |app, text| {
                            let Some(name) = app.accept(BookmarkName::parse(&text)) else {
                                return;
                            };
                            let desc = format!("push {name}");
                            let op = Push {
                                scope: PushScope::Bookmark(name),
                                options,
                            };
                            app.run_bg(desc, move |jj| jj.push(&op));
                        },
                    ));
                }
            }
            TransientAction::PushChange(options) => {
                let rev = self.rev_at_point();
                let desc = format!("push change {rev}");
                let op = Push {
                    scope: PushScope::Change(rev),
                    options,
                };
                self.run_bg(desc, move |jj| jj.push(&op));
            }
            _ => unreachable!("routed by invoke_transient"),
        }
    }
}
