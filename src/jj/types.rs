//! Pure domain types shared by the jj layer and the UI.

use std::sync::Arc;

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileDiff {
    pub path: String,
    /// Set for renames: the pre-rename path.
    pub old_path: Option<String>,
    pub is_new: bool,
    pub is_deleted: bool,
    pub is_binary: bool,
    pub hunks: Vec<Hunk>,
}

impl FileDiff {
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

/// One revision parsed from the log template (see `client::LOG_TEMPLATE`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RevEntry {
    /// Graph prefix preceding the template output ("@  ", "○  ", ""), kept
    /// verbatim for display.
    pub graph: String,
    /// Shortest-unique change id prefix (8 chars).
    pub change_id: String,
    pub commit_id: String,
    /// Space-separated bookmark names ("main main@origin"), possibly empty.
    pub bookmarks: String,
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpEntry {
    pub graph: String,
    pub id: String,
    pub description: String,
    pub time: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpLogRow {
    Op(OpEntry),
    Graph(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookmarkInfo {
    pub name: String,
    /// `None` for local bookmarks, the remote name for remote-tracking ones.
    pub remote: Option<String>,
}

/// Everything the status buffer needs, read in one refresh pass.
#[derive(Debug, Clone, Default)]
pub struct StatusSnapshot {
    /// The working-copy commit (@). `None` only if the read failed oddly.
    pub working_copy: Option<RevEntry>,
    /// Parents of @.
    pub parents: Vec<RevEntry>,
    /// Paths with unresolved conflicts in @.
    pub conflicts: Vec<String>,
    /// Diff of @ against its parents. `Arc`-shared with the status pane,
    /// which needs the same diffs for dispatch — a large diff must not be
    /// deep-copied per refresh.
    pub diff: Arc<Vec<FileDiff>>,
    /// The log section (default revset), graph rows included.
    pub log: Vec<LogRow>,
    /// All bookmarks, for pickers.
    pub bookmarks: Vec<BookmarkInfo>,
}
