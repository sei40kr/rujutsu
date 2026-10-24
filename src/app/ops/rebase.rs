//! Rebase actions: pick a destination, then rebase the revision at point
//! (alone, with descendants, or as a whole branch) onto it.

use crate::jj::op::{EmptiedCommits, Rebase, RebaseKind, Revset};
use crate::jj::types::rev_candidates;

use super::super::{App, Input};

impl App {
    pub(super) fn rebase_action(&mut self, kind: RebaseKind, emptied: EmptiedCommits) {
        let rev = self.rev_at_point();
        let what = match kind {
            RebaseKind::Revision => format!("{rev}"),
            RebaseKind::WithDescendants => format!("{rev} and descendants"),
            RebaseKind::Branch => format!("branch of {rev}"),
        };
        let candidates = rev_candidates(self.snapshot.as_ref())
            .iter()
            .map(ToString::to_string)
            .collect();
        self.input = Some(Input::picker(
            format!("Rebase {what} onto"),
            candidates,
            move |app, dest| {
                let Some(onto) = app.accept(Revset::parse(&dest)) else {
                    return;
                };
                let op = Rebase {
                    kind,
                    rev,
                    onto,
                    emptied,
                };
                app.run_bg(format!("rebase {} onto {}", op.rev, op.onto), move |jj| {
                    jj.rebase(&op)
                });
            },
        ));
    }
}
