//! DWIM commands that act on the section under the cursor: visit, abandon
//! or restore, edit, duplicate, split, evolog, absorb, op-restore. `x`
//! abandons a revision or restores (discards) a working-copy file depending
//! on where point is.

use crate::jj::op::{LogQuery, LogScope, Restore, Revset};
use crate::jj::types::DiffArea;
use crate::keymap::PaneKind;
use crate::ui::section::{Group, SectionValue};

use super::{App, Confirm, PendingAction};

impl App {
    /// Copy the revision the current buffer is about (magit-copy-buffer-revision,
    /// `y b`): the revision a revision buffer shows, else the revision at
    /// point, falling back to `@`.
    pub(super) fn copy_buffer_revision(&mut self) {
        let rev = match self.panes.last() {
            Some(p) if p.kind == PaneKind::Revision => p.title.clone(),
            _ => self.rev_at_point().to_string(),
        };
        self.message = Some(format!("copied: {rev}"));
        self.clipboard_request = Some(rev);
    }

    /// Copy the value under point to the system clipboard (magit-copy-section-value,
    /// `y s`): a change id, an operation id, or a file path. The main loop
    /// performs the copy since it owns the terminal.
    pub(super) fn copy_at_point(&mut self) {
        let Some(pane) = self.panes.last() else {
            return;
        };
        let value = match pane.value_at_cursor() {
            SectionValue::Revision { change_id } => change_id.to_string(),
            SectionValue::Operation { op_id } => op_id.to_string(),
            SectionValue::File { path, .. } | SectionValue::Hunk { path, .. } => path.to_string(),
            _ => {
                self.message = Some("nothing to copy here".into());
                return;
            }
        };
        self.message = Some(format!("copied: {value}"));
        self.clipboard_request = Some(value);
    }

    pub(super) fn visit_at_point(&mut self) {
        let Some(pane) = self.panes.last() else {
            return;
        };
        match pane.value_at_cursor() {
            SectionValue::Revision { change_id } => self.load_revision(change_id.into(), false),
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
                let rev = Revset::from(change_id);
                self.confirm = Some(Confirm {
                    prompt: format!("Abandon revision {rev}?"),
                    action: PendingAction::Jj {
                        desc: format!("abandon {rev}"),
                        run: Box::new(move |jj| jj.abandon(&rev)),
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
                        run: Box::new(move |jj| jj.restore(&Restore::Path(path))),
                    },
                });
            }
            SectionValue::Group(Group::Changes) => {
                self.confirm = Some(Confirm {
                    prompt: "Restore the whole working copy (discard all changes)?".into(),
                    action: PendingAction::Jj {
                        desc: "restore working copy".into(),
                        run: Box::new(|jj| jj.restore(&Restore::WorkingCopy)),
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

    /// The revision at point, or a "no revision at point" message.
    fn revision_at_point(&mut self) -> Option<Revset> {
        match self.panes.last().map(|p| p.value_at_cursor()) {
            Some(SectionValue::Revision { change_id }) => Some(change_id.into()),
            _ => {
                self.message = Some("no revision at point".into());
                None
            }
        }
    }

    /// `e`: make the revision at point the working copy (`jj edit`).
    pub(super) fn edit_at_point(&mut self) {
        if let Some(rev) = self.revision_at_point() {
            self.run_bg(format!("edit {rev}"), move |jj| jj.edit(&rev));
        }
    }

    pub(super) fn duplicate_at_point(&mut self) {
        if let Some(rev) = self.revision_at_point() {
            self.run_bg(format!("duplicate {rev}"), move |jj| jj.duplicate(&rev));
        }
    }

    /// `a`: move each working-copy change into the mutable ancestor that
    /// last touched those lines.
    pub(super) fn absorb(&mut self) {
        self.run_bg("absorb", |jj| jj.absorb());
    }

    /// `S`: split the revision at point (or @). Interactive — jj opens the
    /// configured diff editor, so the terminal is handed over.
    pub(super) fn split_at_point(&mut self) {
        let rev = self.rev_at_point();
        self.request_editor(format!("split {rev}"), move |jj| jj.split_in_editor(&rev));
    }

    /// `v`: how the revision at point evolved (`jj evolog`).
    pub(super) fn evolog_at_point(&mut self) {
        let rev = self.rev_at_point();
        let query = LogQuery {
            scope: LogScope::Evolution(rev.clone()),
            limit: None,
        };
        self.load_log(format!("evolog of {rev}"), query, false);
    }

    /// `r` in the op-log buffer: roll the repo back to the operation at
    /// point (itself undoable, but still confirmed).
    pub(super) fn op_restore_at_point(&mut self) {
        match self.panes.last().map(|p| p.value_at_cursor()) {
            Some(SectionValue::Operation { op_id: op }) => {
                self.confirm = Some(Confirm {
                    prompt: format!("Restore the repo to operation {op}?"),
                    action: PendingAction::Jj {
                        desc: format!("op restore {op}"),
                        run: Box::new(move |jj| jj.op_restore(&op)),
                    },
                });
            }
            _ => self.message = Some("no operation at point".into()),
        }
    }
}
