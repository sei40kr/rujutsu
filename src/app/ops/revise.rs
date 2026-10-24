//! Describe / commit / new / squash actions. Anything that may open
//! $EDITOR (describe, commit, squash's message merging) is handed to the
//! main loop as an `EditorRequest`, which suspends the TUI for the
//! duration; the rest runs in the background.

use crate::jj::op::{
    Commit, Describe, NewChange, Placement, Revset, Squash, SquashDescription, SquashMessage,
    SquashOptions, SquashSource,
};
use crate::jj::types::DiffArea;
use crate::ui::section::SectionValue;
use crate::ui::transient::TransientAction;

use super::super::App;

/// Squash DWIM: on a working-copy file/hunk squash just that path into the
/// parent (Magit's stage-like gesture); on a revision squash it into its
/// own parent; anywhere else squash @.
fn squash_source(value: SectionValue) -> SquashSource {
    match value {
        SectionValue::File {
            area: DiffArea::WorkingCopy,
            path,
        }
        | SectionValue::Hunk {
            area: DiffArea::WorkingCopy,
            path,
            ..
        } => SquashSource::WorkingCopyPath(path),
        SectionValue::Revision { change_id } => SquashSource::Revision(change_id.into()),
        _ => SquashSource::Revision(Revset::working_copy()),
    }
}

impl App {
    pub(super) fn revise_action(&mut self, action: TransientAction) {
        let rev = self.rev_at_point();
        match action {
            TransientAction::Describe(author) => {
                let op = Describe { rev, author };
                self.request_editor(format!("describe {}", op.rev), move |jj| {
                    jj.describe_in_editor(&op)
                });
            }
            TransientAction::Commit(author) => {
                let op = Commit { author };
                self.request_editor("commit", move |jj| jj.commit_in_editor(&op));
            }
            TransientAction::New(placement, checkout) => {
                let desc = match placement {
                    Placement::ChildOf => format!("new child of {rev}"),
                    Placement::After => format!("new after {rev}"),
                    Placement::Before => format!("new before {rev}"),
                };
                let op = NewChange {
                    anchor: rev,
                    placement,
                    checkout,
                };
                self.run_bg(desc, move |jj| jj.new_change(&op));
            }
            TransientAction::Squash(options) => {
                let value = self
                    .panes
                    .last()
                    .map(|p| p.value_at_cursor())
                    .unwrap_or(SectionValue::Root);
                let source = squash_source(value);
                let desc = match &source {
                    SquashSource::WorkingCopyPath(p) => format!("squash {p}"),
                    SquashSource::Revision(r) => format!("squash {r}"),
                    SquashSource::WorkingCopyInto(r) => format!("squash into {r}"),
                };
                self.squash(desc, source, options);
            }
            TransientAction::SquashInto(options) => {
                // Move the working copy's changes into the revision at point.
                let desc = format!("squash into {rev}");
                self.squash(desc, SquashSource::WorkingCopyInto(rev), options);
            }
            _ => unreachable!("routed by invoke_transient"),
        }
    }

    /// Keeping the destination's description needs no editor, so that
    /// variant runs in the background; combining descriptions hands over
    /// the terminal.
    fn squash(&mut self, desc: String, source: SquashSource, options: SquashOptions) {
        let op = Squash {
            source,
            emptied: options.emptied,
        };
        match options.description {
            SquashDescription::UseDestination => self.run_bg(desc, move |jj| {
                jj.squash(&op, &SquashMessage::UseDestination)
            }),
            SquashDescription::Combine => {
                self.request_editor(desc, move |jj| jj.squash_in_editor(&op))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jj::op::{ChangeId, RepoPath};
    use crate::ui::section::Group;

    #[test]
    fn squash_source_follows_point() {
        let file = SectionValue::File {
            area: DiffArea::WorkingCopy,
            path: RepoPath::parse("a.txt").unwrap(),
        };
        assert_eq!(
            squash_source(file),
            SquashSource::WorkingCopyPath(RepoPath::parse("a.txt").unwrap())
        );
        let rev = SectionValue::Revision {
            change_id: ChangeId::parse("xyz").unwrap(),
        };
        assert_eq!(
            squash_source(rev),
            SquashSource::Revision(Revset::parse("xyz").unwrap())
        );
        // A committed (read-only) diff is not a squash source: fall back to @.
        let committed = SectionValue::File {
            area: DiffArea::Committed,
            path: RepoPath::parse("a.txt").unwrap(),
        };
        assert_eq!(
            squash_source(committed),
            SquashSource::Revision(Revset::working_copy())
        );
        assert_eq!(
            squash_source(SectionValue::Group(Group::Changes)),
            SquashSource::Revision(Revset::working_copy())
        );
    }
}
