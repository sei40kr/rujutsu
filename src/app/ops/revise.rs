//! Describe / commit / new / squash actions. Anything that may open
//! $EDITOR (describe, commit, squash's message merging) is handed to the
//! main loop as an `EditorRequest`, which suspends the TUI for the
//! duration; the rest runs in the background.

use crate::jj::types::DiffArea;
use crate::ui::section::SectionValue;
use crate::ui::transient::TransientAction;

use super::super::{App, EditorRequest};

impl App {
    pub(super) fn revise_action(&mut self, action: TransientAction, args: Vec<String>) {
        let rev = self.rev_at_point();
        match action {
            TransientAction::Describe => {
                let mut full = vec!["describe".to_string(), rev.clone()];
                full.extend(args);
                self.request_editor(EditorRequest::new(format!("describe {rev}"), full));
            }
            TransientAction::Commit => {
                let mut full = vec!["commit".to_string()];
                full.extend(args);
                self.request_editor(EditorRequest::new("commit", full));
            }
            TransientAction::NewChild => {
                let mut full = vec!["new".to_string(), rev.clone()];
                full.extend(args);
                self.run_jj_bg(format!("new child of {rev}"), full);
            }
            TransientAction::NewAfter => {
                let mut full = vec!["new".to_string(), "--insert-after".to_string(), rev.clone()];
                full.extend(args);
                self.run_jj_bg(format!("new after {rev}"), full);
            }
            TransientAction::NewBefore => {
                let mut full = vec![
                    "new".to_string(),
                    "--insert-before".to_string(),
                    rev.clone(),
                ];
                full.extend(args);
                self.run_jj_bg(format!("new before {rev}"), full);
            }
            TransientAction::Squash => self.squash_dwim(args),
            TransientAction::SquashInto => {
                // Move the working copy's changes into the revision at point.
                let mut full = vec!["squash".to_string(), "--into".to_string(), rev.clone()];
                full.extend(args);
                self.request_editor(EditorRequest::new(format!("squash into {rev}"), full));
            }
            _ => unreachable!("routed by invoke_transient"),
        }
    }

    /// Squash DWIM: on a working-copy file/hunk squash just that path into
    /// the parent (Magit's stage-like gesture); on a revision squash it into
    /// its own parent; anywhere else squash @.
    fn squash_dwim(&mut self, args: Vec<String>) {
        let value = self
            .panes
            .last()
            .map(|p| p.value_at_cursor())
            .unwrap_or(SectionValue::Root);
        let (desc, mut full) = match value {
            SectionValue::File {
                area: DiffArea::WorkingCopy,
                path,
            }
            | SectionValue::Hunk {
                area: DiffArea::WorkingCopy,
                path,
                ..
            } => (
                format!("squash {path}"),
                vec!["squash".to_string(), "--".to_string(), path],
            ),
            SectionValue::Revision { change_id } => (
                format!("squash {change_id}"),
                vec!["squash".to_string(), "-r".to_string(), change_id],
            ),
            _ => (
                "squash @".to_string(),
                vec!["squash".to_string(), "-r".to_string(), "@".to_string()],
            ),
        };
        // Path arguments must stay last: insert the switches before "--".
        let insert_at = full.iter().position(|a| a == "--").unwrap_or(full.len());
        for (i, a) in args.into_iter().enumerate() {
            full.insert(insert_at + i, a);
        }
        self.request_editor(EditorRequest::new(desc, full));
    }
}
