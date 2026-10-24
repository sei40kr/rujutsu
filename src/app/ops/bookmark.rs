//! Bookmark actions: create/set/delete/rename/track/untrack/forget. Names
//! typed or picked in the minibuffer are parsed into `BookmarkName` /
//! `RemoteBookmark` here, so a malformed one is reported before jj sees it.

use crate::jj::op::{BookmarkName, BookmarkOp, BookmarkRef, RemoteBookmark};
use crate::ui::transient::TransientAction;

use super::super::{App, Confirm, Input, PendingAction};

impl App {
    /// Local bookmark names, for pickers.
    fn local_bookmarks(&self) -> Vec<String> {
        self.snapshot
            .as_ref()
            .map(|s| {
                s.bookmarks
                    .iter()
                    .filter(|b| matches!(b, BookmarkRef::Local(_)))
                    .map(ToString::to_string)
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
                    .filter(|b| matches!(b, BookmarkRef::Remote(_)))
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn bookmark_bg(&mut self, desc: String, op: BookmarkOp) {
        self.run_bg(desc, move |jj| jj.bookmark(&op));
    }

    fn bookmark_confirm(&mut self, prompt: String, desc: String, op: BookmarkOp) {
        self.confirm = Some(Confirm {
            prompt,
            action: PendingAction::Jj {
                desc,
                run: Box::new(move |jj| jj.bookmark(&op)),
            },
        });
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
                move |app, text| {
                    if let Some(name) = app.accept(BookmarkName::parse(&text)) {
                        app.bookmark_bg(
                            format!("create bookmark {name}"),
                            BookmarkOp::Create { name, at: rev },
                        );
                    }
                },
            )),
            TransientAction::BookmarkSet => self
                .bookmark_picker(self.local_bookmarks(), "local bookmarks")
                .map(|cands| {
                    Input::picker(
                        format!("Move bookmark to {rev}"),
                        cands,
                        move |app, text| {
                            if let Some(name) = app.accept(BookmarkName::parse(&text)) {
                                app.bookmark_bg(
                                    format!("move bookmark {name} to {rev}"),
                                    BookmarkOp::Set { name, at: rev },
                                );
                            }
                        },
                    )
                }),
            TransientAction::BookmarkDelete => self
                .bookmark_picker(self.local_bookmarks(), "local bookmarks")
                .map(|cands| {
                    Input::picker("Delete bookmark", cands, |app, text| {
                        if let Some(name) = app.accept(BookmarkName::parse(&text)) {
                            app.bookmark_confirm(
                                format!("Delete bookmark {name}?"),
                                format!("delete bookmark {name}"),
                                BookmarkOp::Delete(name),
                            );
                        }
                    })
                }),
            TransientAction::BookmarkRename => self
                .bookmark_picker(self.local_bookmarks(), "local bookmarks")
                .map(|cands| {
                    Input::picker("Rename bookmark", cands, |app, text| {
                        let Some(from) = app.accept(BookmarkName::parse(&text)) else {
                            return;
                        };
                        // Second prompt: the new name, with the old one closed over.
                        app.input = Some(Input::plain(
                            format!("Rename {from} to"),
                            move |app, text| {
                                if let Some(to) = app.accept(BookmarkName::parse(&text)) {
                                    app.bookmark_bg(
                                        format!("rename bookmark {from} to {to}"),
                                        BookmarkOp::Rename { from, to },
                                    );
                                }
                            },
                        ));
                    })
                }),
            TransientAction::BookmarkTrack => self
                .bookmark_picker(self.remote_bookmarks(), "remote bookmarks")
                .map(|cands| {
                    Input::picker("Track remote bookmark (name@remote)", cands, |app, text| {
                        if let Some(b) = app.accept(RemoteBookmark::parse(&text)) {
                            app.bookmark_bg(format!("track {b}"), BookmarkOp::Track(b));
                        }
                    })
                }),
            TransientAction::BookmarkUntrack => self
                .bookmark_picker(self.remote_bookmarks(), "remote bookmarks")
                .map(|cands| {
                    Input::picker(
                        "Untrack remote bookmark (name@remote)",
                        cands,
                        |app, text| {
                            if let Some(b) = app.accept(RemoteBookmark::parse(&text)) {
                                app.bookmark_bg(format!("untrack {b}"), BookmarkOp::Untrack(b));
                            }
                        },
                    )
                }),
            TransientAction::BookmarkForget => self
                .bookmark_picker(self.local_bookmarks(), "local bookmarks")
                .map(|cands| {
                    Input::picker("Forget bookmark", cands, |app, text| {
                        if let Some(name) = app.accept(BookmarkName::parse(&text)) {
                            app.bookmark_confirm(
                                format!("Forget bookmark {name}?"),
                                format!("forget bookmark {name}"),
                                BookmarkOp::Forget(name),
                            );
                        }
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
