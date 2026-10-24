//! Pure domain types shared by the jj layer and the UI. Ids, names and
//! paths are the `op` value objects, so what the parser hands out can go
//! straight back into an operation.

use std::sync::Arc;

use super::op::{BookmarkRef, ChangeId, CommitId, OperationId, RepoPath, Revset};

/// Which diff a file or hunk belongs to. Commands dispatch on this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiffArea {
    /// The working-copy commit's diff (status buffer) — restorable.
    WorkingCopy,
    /// Read-only diffs (revision buffers).
    Committed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    /// The full `@@ -a,b +c,d @@ ctx` line as emitted by `jj diff --git`.
    pub header: String,
    /// Content lines including their `' '`/`'+'`/`'-'`/`'\\'` prefix.
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub path: RepoPath,
    /// Set for renames: the pre-rename path.
    pub old_path: Option<RepoPath>,
    pub is_new: bool,
    pub is_deleted: bool,
    pub is_binary: bool,
    pub hunks: Vec<Hunk>,
}

impl FileDiff {
    /// An unchanged-looking entry for `path`; the parser fills in the rest.
    pub fn new(path: RepoPath) -> Self {
        Self {
            path,
            old_path: None,
            is_new: false,
            is_deleted: false,
            is_binary: false,
            hunks: Vec::new(),
        }
    }

    pub fn status_word(&self) -> &'static str {
        if self.is_new {
            "new file"
        } else if self.is_deleted {
            "deleted "
        } else if self.old_path.is_some() {
            "renamed "
        } else {
            "modified"
        }
    }
}

/// One revision parsed from the log template (see `cli::LOG_TEMPLATE`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevEntry {
    /// Graph prefix preceding the template output ("@  ", "○  ", ""), kept
    /// verbatim for display.
    pub graph: String,
    /// Shortest-unique prefix (8 chars).
    pub change_id: ChangeId,
    pub commit_id: CommitId,
    /// Length of the unique prefix of `change_id` / `commit_id`.
    pub change_id_prefix_len: usize,
    pub commit_id_prefix_len: usize,
    pub bookmarks: Vec<BookmarkRef>,
    /// Space-separated tag names, possibly empty.
    pub tags: String,
    /// First line of the description; empty when undescribed.
    pub subject: String,
    pub author: String,
    /// Relative committer timestamp, e.g. "3 days ago".
    pub date: String,
    pub is_working_copy: bool,
    pub has_conflict: bool,
    pub is_empty: bool,
    pub is_immutable: bool,
    pub is_root: bool,
    pub is_divergent: bool,
    pub is_hidden: bool,
}

/// One line of `jj log` graph output: either a revision row or a pure
/// graph line ("│", "├─╯", "~" for elided revisions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogRow {
    Rev(RevEntry),
    Graph(String),
}

/// One operation parsed from the op-log template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpEntry {
    pub graph: String,
    pub id: OperationId,
    pub description: String,
    pub time: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpLogRow {
    Op(OpEntry),
    Graph(String),
}

/// What a revision buffer shows: jj's header text and the parsed diff.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RevisionView {
    pub header: String,
    pub files: Vec<FileDiff>,
}

/// Everything the status buffer needs, read in one refresh pass.
#[derive(Debug, Clone, Default)]
pub struct StatusSnapshot {
    /// The working-copy commit (@). `None` only if the read failed oddly.
    pub working_copy: Option<RevEntry>,
    /// Parents of @.
    pub parents: Vec<RevEntry>,
    /// Paths with unresolved conflicts in @.
    pub conflicts: Vec<RepoPath>,
    /// Diff of @ against its parents. `Arc`-shared with the status pane,
    /// which needs the same diffs for dispatch — a large diff must not be
    /// deep-copied per refresh.
    pub diff: Arc<Vec<FileDiff>>,
    /// The log section (default revset), graph rows included.
    pub log: Vec<LogRow>,
    /// All bookmarks, for pickers.
    pub bookmarks: Vec<BookmarkRef>,
}

/// Bookmarks plus a few revset symbols worth offering for completion.
pub fn rev_candidates(snapshot: Option<&StatusSnapshot>) -> Vec<Revset> {
    let mut out = vec![
        Revset::working_copy(),
        Revset::working_copy().parents(),
        Revset::parse("trunk()").expect("literal revset"),
    ];
    if let Some(s) = snapshot {
        for b in &s.bookmarks {
            let r = Revset::parse(&b.to_string()).expect("bookmark refs are non-empty");
            if !out.contains(&r) {
                out.push(r);
            }
        }
    }
    out
}
