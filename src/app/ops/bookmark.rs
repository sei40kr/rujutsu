//! Bookmark actions: create/set/delete/rename/track/untrack/forget.

use crate::ui::transient::TransientAction;

use super::super::{svec, App, Confirm, Input, PendingAction};

impl App {
    /// Local bookmark names, for pickers.
    fn local_bookmarks(&self) -> Vec<String> {
        self.snapshot
            .as_ref()
            .map(|s| {
                s.bookmarks
                    .iter()
                    .filter(|b| b.remote.is_none())
                    .map(|b| b.name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// `name@remote` candidates, for track/untrack pickers.
    fn remote_bookmarks(&self) -> Vec<String> {
        self.snapshot
            .as_ref()
            .map(|s| {
                s.bookmarks
                    .iter()
                    .filter_map(|b| {
                        b.remote
                            .as_ref()
                            .map(|remote| format!("{}@{}", b.name, remote))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(super) fn bookmark_action(&mut self, action: TransientAction) {
        let rev = self.rev_at_point();
        // Bookmark create takes a brand-new name, so it is a plain prompt.
        // Every other action operates on an existing bookmark; when there are
        // none to pick from, opening an empty input just invites a name that
        // will fail, so tell the user instead.
        let input = match action {
            TransientAction::BookmarkCreate => Some(Input::plain(
                format!("Create bookmark at {rev}"),
                move |app, name| {
                    app.run_jj_bg(
                        format!("create bookmark {name}"),
                        svec(&["bookmark", "create", &name, "-r", &rev]),
                    );
                },
            )),
            TransientAction::BookmarkSet => self
                .bookmark_picker(self.local_bookmarks(), "local bookmarks")
                .map(|cands| {
                    Input::picker(
                        format!("Move bookmark to {rev}"),
                        cands,
                        move |app, name| {
                            app.run_jj_bg(
                                format!("move bookmark {name} to {rev}"),
                                svec(&["bookmark", "set", &name, "-r", &rev]),
                            );
                        },
                    )
                }),
            TransientAction::BookmarkDelete => self
                .bookmark_picker(self.local_bookmarks(), "local bookmarks")
                .map(|cands| {
                    Input::picker("Delete bookmark", cands, |app, name| {
                        app.confirm = Some(Confirm {
                            prompt: format!("Delete bookmark {name}?"),
                            action: PendingAction::Jj {
                                desc: format!("delete bookmark {name}"),
                                args: svec(&["bookmark", "delete", &name]),
                            },
                        });
                    })
                }),
            TransientAction::BookmarkRename => self
                .bookmark_picker(self.local_bookmarks(), "local bookmarks")
                .map(|cands| {
                    Input::picker("Rename bookmark", cands, |app, old| {
                        // Second prompt: the new name, with the old one closed over.
                        app.input =
                            Some(Input::plain(format!("Rename {old} to"), move |app, new| {
                                app.run_jj_bg(
                                    format!("rename bookmark {old} to {new}"),
                                    svec(&["bookmark", "rename", &old, &new]),
                                );
                            }));
                    })
                }),
            TransientAction::BookmarkTrack => self
                .bookmark_picker(self.remote_bookmarks(), "remote bookmarks")
                .map(|cands| {
                    Input::picker("Track remote bookmark (name@remote)", cands, |app, name| {
                        app.run_jj_bg(format!("track {name}"), svec(&["bookmark", "track", &name]));
                    })
                }),
            TransientAction::BookmarkUntrack => self
                .bookmark_picker(self.remote_bookmarks(), "remote bookmarks")
                .map(|cands| {
                    Input::picker(
                        "Untrack remote bookmark (name@remote)",
                        cands,
                        |app, name| {
                            app.run_jj_bg(
                                format!("untrack {name}"),
                                svec(&["bookmark", "untrack", &name]),
                            );
                        },
                    )
                }),
            TransientAction::BookmarkForget => self
                .bookmark_picker(self.local_bookmarks(), "local bookmarks")
                .map(|cands| {
                    Input::picker("Forget bookmark", cands, |app, name| {
                        app.confirm = Some(Confirm {
                            prompt: format!("Forget bookmark {name}?"),
                            action: PendingAction::Jj {
                                desc: format!("forget bookmark {name}"),
                                args: svec(&["bookmark", "forget", &name]),
                            },
                        });
                    })
                }),
            _ => unreachable!("routed by invoke_transient"),
        };
        self.input = input;
    }

    /// The picker candidates, or `None` (with a "no bookmarks" message) when
    /// there is nothing to pick from. `kind` names the empty set for the
    /// message (e.g. "local bookmarks").
    fn bookmark_picker(&mut self, candidates: Vec<String>, kind: &str) -> Option<Vec<String>> {
        if candidates.is_empty() {
            self.message = Some(format!("no {kind}"));
            None
        } else {
            Some(candidates)
        }
    }
}
