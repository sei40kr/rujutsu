//! DWIM commands that act on the section under the cursor: visit, abandon
//! or restore, edit, duplicate, split, evolog, absorb, op-restore. `x`
//! abandons a revision or restores (discards) a working-copy file depending
//! on where point is.

use crate::jj::types::DiffArea;
use crate::ui::section::{Group, SectionValue};

use super::{svec, App, Confirm, PendingAction};

impl App {
    pub(super) fn visit_at_point(&mut self) {
        let Some(pane) = self.panes.last() else {
            return;
        };
        match pane.value_at_cursor() {
            SectionValue::Revision { change_id } => self.load_revision(change_id, false),
            SectionValue::Operation { op_id } => self.load_op_show(op_id),
            _ => self.message = Some("nothing to visit here".into()),
        }
    }

    /// `x`: abandon the revision at point, or restore the file/hunk's file
    /// at point (both destructive — confirmed first).
    pub(super) fn abandon_or_restore_at_point(&mut self) {
        let Some(pane) = self.panes.last() else {
            return;
        };
        match pane.value_at_cursor() {
            SectionValue::Revision { change_id } => {
                self.confirm = Some(Confirm {
                    prompt: format!("Abandon revision {change_id}?"),
                    action: PendingAction::Jj {
                        desc: format!("abandon {change_id}"),
                        args: svec(&["abandon", "-r", &change_id]),
                    },
                });
            }
            SectionValue::File {
                area: DiffArea::WorkingCopy,
                path,
            }
            | SectionValue::Hunk {
                area: DiffArea::WorkingCopy,
                path,
                ..
            } => {
                self.confirm = Some(Confirm {
                    prompt: format!("Restore {path} (discard its changes)?"),
                    action: PendingAction::Jj {
                        desc: format!("restore {path}"),
                        args: svec(&["restore", "--", &path]),
                    },
                });
            }
            SectionValue::Group(Group::Changes) => {
                self.confirm = Some(Confirm {
                    prompt: "Restore the whole working copy (discard all changes)?".into(),
                    action: PendingAction::Jj {
                        desc: "restore working copy".into(),
                        args: svec(&["restore"]),
                    },
                });
            }
            SectionValue::File {
                area: DiffArea::Committed,
                ..
            }
            | SectionValue::Hunk {
                area: DiffArea::Committed,
                ..
            } => {
                self.message = Some("read-only diff — abandon the revision instead".into());
            }
            _ => self.message = Some("nothing to abandon or restore here".into()),
        }
    }

    /// `e`: make the revision at point the working copy (`jj edit`).
    pub(super) fn edit_at_point(&mut self) {
        match self.panes.last().map(|p| p.value_at_cursor()) {
            Some(SectionValue::Revision { change_id }) => {
                self.run_jj_bg(
                    format!("edit {change_id}"),
                    svec(&["edit", "-r", &change_id]),
                );
            }
            _ => self.message = Some("no revision at point".into()),
        }
    }

    pub(super) fn duplicate_at_point(&mut self) {
        match self.panes.last().map(|p| p.value_at_cursor()) {
            Some(SectionValue::Revision { change_id }) => {
                self.run_jj_bg(
                    format!("duplicate {change_id}"),
                    svec(&["duplicate", &change_id]),
                );
            }
            _ => self.message = Some("no revision at point".into()),
        }
    }

    /// `a`: move each working-copy change into the mutable ancestor that
    /// last touched those lines.
    pub(super) fn absorb(&mut self) {
        self.run_jj_bg("absorb".into(), svec(&["absorb"]));
    }

    /// `S`: split the revision at point (or @). Interactive — jj opens the
    /// configured diff editor, so the terminal is handed over.
    pub(super) fn split_at_point(&mut self) {
        let rev = self.rev_at_point();
        self.request_editor(super::EditorRequest::new(
            format!("split {rev}"),
            svec(&["split", "-r", &rev]),
        ));
    }

    /// `v`: how the revision at point evolved (`jj evolog`).
    pub(super) fn evolog_at_point(&mut self) {
        let rev = self.rev_at_point();
        let args = svec(&["evolog", "-r", &rev]);
        self.load_log(format!("evolog of {rev}"), args, false);
    }

    /// `r` in the op-log buffer: roll the repo back to the operation at
    /// point (itself undoable, but still confirmed).
    pub(super) fn op_restore_at_point(&mut self) {
        match self.panes.last().map(|p| p.value_at_cursor()) {
            Some(SectionValue::Operation { op_id }) => {
                self.confirm = Some(Confirm {
                    prompt: format!("Restore the repo to operation {op_id}?"),
                    action: PendingAction::Jj {
                        desc: format!("op restore {op_id}"),
                        args: svec(&["op", "restore", &op_id]),
                    },
                });
            }
            _ => self.message = Some("no operation at point".into()),
        }
    }
}
