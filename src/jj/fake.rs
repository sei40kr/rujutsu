//! A `Jj` that records what it was asked instead of running anything, so
//! app-level tests can assert on the operation the UI built.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::op::{
    BookmarkOp, Commit, Describe, Fetch, LogQuery, Message, NewChange, OperationId, Push, Rebase,
    Restore, Revset, Squash, SquashMessage,
};
use super::types::{LogRow, OpLogRow, RevisionView, StatusSnapshot};
use super::{Jj, JjError, Outcome};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    Log(LogQuery),
    Describe(Describe, Option<Message>),
    Commit(Commit, Option<Message>),
    NewChange(NewChange),
    Squash(Squash, Option<SquashMessage>),
    Rebase(Rebase),
    Bookmark(BookmarkOp),
    Push(Push),
    Fetch(Fetch),
    Abandon(Revset),
    Restore(Restore),
    Edit(Revset),
    Duplicate(Revset),
    Absorb,
    Undo,
    Redo,
    OpRestore(OperationId),
    Split(Revset),
}

#[derive(Default)]
pub struct FakeJj {
    root: PathBuf,
    calls: Arc<Mutex<Vec<Call>>>,
}

impl FakeJj {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }

    fn record(&self, call: Call) -> Result<Outcome, JjError> {
        self.calls.lock().unwrap().push(call);
        Ok(Outcome {
            cmd: "jj (fake)".into(),
            status: 0,
            output: String::new(),
        })
    }
}

impl Jj for FakeJj {
    fn workspace_root(&self) -> &Path {
        &self.root
    }
    fn snapshot(&self, _: Option<&Revset>) -> Result<StatusSnapshot, JjError> {
        Ok(StatusSnapshot::default())
    }
    fn log(&self, query: &LogQuery) -> Result<Vec<LogRow>, JjError> {
        self.calls.lock().unwrap().push(Call::Log(query.clone()));
        Ok(Vec::new())
    }
    fn revision(&self, _: &Revset) -> Result<RevisionView, JjError> {
        Ok(RevisionView::default())
    }
    fn op_log(&self) -> Result<Vec<OpLogRow>, JjError> {
        Ok(Vec::new())
    }
    fn op_show(&self, _: &OperationId) -> Result<String, JjError> {
        Ok(String::new())
    }
    fn describe(&self, op: &Describe, msg: &Message) -> Result<Outcome, JjError> {
        self.record(Call::Describe(op.clone(), Some(msg.clone())))
    }
    fn commit(&self, op: &Commit, msg: &Message) -> Result<Outcome, JjError> {
        self.record(Call::Commit(op.clone(), Some(msg.clone())))
    }
    fn new_change(&self, op: &NewChange) -> Result<Outcome, JjError> {
        self.record(Call::NewChange(op.clone()))
    }
    fn squash(&self, op: &Squash, msg: &SquashMessage) -> Result<Outcome, JjError> {
        self.record(Call::Squash(op.clone(), Some(msg.clone())))
    }
    fn rebase(&self, op: &Rebase) -> Result<Outcome, JjError> {
        self.record(Call::Rebase(op.clone()))
    }
    fn bookmark(&self, op: &BookmarkOp) -> Result<Outcome, JjError> {
        self.record(Call::Bookmark(op.clone()))
    }
    fn push(&self, op: &Push) -> Result<Outcome, JjError> {
        self.record(Call::Push(op.clone()))
    }
    fn fetch(&self, op: &Fetch) -> Result<Outcome, JjError> {
        self.record(Call::Fetch(*op))
    }
    fn abandon(&self, rev: &Revset) -> Result<Outcome, JjError> {
        self.record(Call::Abandon(rev.clone()))
    }
    fn restore(&self, op: &Restore) -> Result<Outcome, JjError> {
        self.record(Call::Restore(op.clone()))
    }
    fn edit(&self, rev: &Revset) -> Result<Outcome, JjError> {
        self.record(Call::Edit(rev.clone()))
    }
    fn duplicate(&self, rev: &Revset) -> Result<Outcome, JjError> {
        self.record(Call::Duplicate(rev.clone()))
    }
    fn absorb(&self) -> Result<Outcome, JjError> {
        self.record(Call::Absorb)
    }
    fn undo(&self) -> Result<Outcome, JjError> {
        self.record(Call::Undo)
    }
    fn redo(&self) -> Result<Outcome, JjError> {
        self.record(Call::Redo)
    }
    fn op_restore(&self, op: &OperationId) -> Result<Outcome, JjError> {
        self.record(Call::OpRestore(op.clone()))
    }
    fn describe_in_editor(&self, op: &Describe) -> Result<Outcome, JjError> {
        self.record(Call::Describe(op.clone(), None))
    }
    fn commit_in_editor(&self, op: &Commit) -> Result<Outcome, JjError> {
        self.record(Call::Commit(op.clone(), None))
    }
    fn squash_in_editor(&self, op: &Squash) -> Result<Outcome, JjError> {
        self.record(Call::Squash(op.clone(), None))
    }
    fn split_in_editor(&self, rev: &Revset) -> Result<Outcome, JjError> {
        self.record(Call::Split(rev.clone()))
    }
}
