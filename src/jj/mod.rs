//! The jj layer. `op` says *what* the app wants in jj's vocabulary, `Jj` is
//! the port the app talks to, and `cli` is the one adapter: it translates
//! each operation into a `jj` command line and shells out. `parse` and
//! `types` are the pure data side the adapter and the UI share.

pub mod cli;
pub mod op;
pub mod parse;
pub mod types;

#[cfg(test)]
pub mod fake;

use std::path::Path;

use op::{
    BookmarkOp, Commit, Describe, DiffTool, Fetch, LogQuery, Message, NewChange, OperationId, Push,
    Rebase, Restore, Revset, Squash, SquashMessage,
};
use types::{LogRow, OpLogRow, RevisionView, StatusSnapshot};

#[derive(Debug, thiserror::Error)]
pub enum JjError {
    #[error("failed to run jj: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("jj {cmd} failed ({code}): {stderr}")]
    Failed {
        cmd: String,
        code: i32,
        stderr: String,
    },
}

/// One executed mutation: what ran, how it exited, what it printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub cmd: String,
    pub status: i32,
    pub output: String,
}

impl Outcome {
    pub fn ok(&self) -> bool {
        self.status == 0
    }
}

/// Everything the app needs from jj. Mutations report a non-zero exit via
/// `Outcome::status`, not `Err`; `Err` means jj could not be run at all.
///
/// The `*_in_editor` methods hand the terminal to jj (which opens `$EDITOR`
/// or the diff editor); the caller must have released it first.
pub trait Jj: Send + Sync {
    fn workspace_root(&self) -> &Path;

    // ---- reads (never snapshot, except `snapshot` itself) ----
    /// Everything the status buffer shows. `log_revset` overrides jj's
    /// default revset for the log section.
    fn snapshot(&self, log_revset: Option<&Revset>) -> Result<StatusSnapshot, JjError>;
    fn log(&self, query: &LogQuery) -> Result<Vec<LogRow>, JjError>;
    fn revision(&self, rev: &Revset) -> Result<RevisionView, JjError>;
    fn op_log(&self) -> Result<Vec<OpLogRow>, JjError>;
    fn op_show(&self, op: &OperationId) -> Result<String, JjError>;

    // ---- headless mutations ----
    fn describe(&self, op: &Describe, msg: &Message) -> Result<Outcome, JjError>;
    fn commit(&self, op: &Commit, msg: &Message) -> Result<Outcome, JjError>;
    fn new_change(&self, op: &NewChange) -> Result<Outcome, JjError>;
    fn squash(&self, op: &Squash, msg: &SquashMessage) -> Result<Outcome, JjError>;
    fn rebase(&self, op: &Rebase) -> Result<Outcome, JjError>;
    fn bookmark(&self, op: &BookmarkOp) -> Result<Outcome, JjError>;
    fn push(&self, op: &Push) -> Result<Outcome, JjError>;
    fn fetch(&self, op: &Fetch) -> Result<Outcome, JjError>;
    fn abandon(&self, rev: &Revset) -> Result<Outcome, JjError>;
    fn restore(&self, op: &Restore) -> Result<Outcome, JjError>;
    fn edit(&self, rev: &Revset) -> Result<Outcome, JjError>;
    fn duplicate(&self, rev: &Revset) -> Result<Outcome, JjError>;
    fn absorb(&self) -> Result<Outcome, JjError>;
    fn undo(&self) -> Result<Outcome, JjError>;
    fn redo(&self) -> Result<Outcome, JjError>;
    fn op_restore(&self, op: &OperationId) -> Result<Outcome, JjError>;

    // ---- interactive mutations ----
    fn describe_in_editor(&self, op: &Describe) -> Result<Outcome, JjError>;
    fn commit_in_editor(&self, op: &Commit) -> Result<Outcome, JjError>;
    fn squash_in_editor(&self, op: &Squash) -> Result<Outcome, JjError>;
    fn split_in_editor(&self, rev: &Revset) -> Result<Outcome, JjError>;
    /// `restore` choosing hunks in a diff editor (`tool`, or jj's default).
    fn restore_in_editor(&self, op: &Restore, tool: Option<&DiffTool>) -> Result<Outcome, JjError>;
}
