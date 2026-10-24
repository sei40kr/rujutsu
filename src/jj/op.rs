//! What the app asks jj to do, in jj's own vocabulary (changes, revisions,
//! bookmarks, the working copy). Nothing here knows about command lines;
//! the `Jj` port takes these values and an adapter translates them.
//!
//! Strings that name things are value objects with a validating `parse`,
//! so a bad name is rejected where the user typed it, not by jj. Choices
//! are enums rather than bools so a call site reads as intent.

use std::fmt;
use std::num::NonZeroUsize;

/// A value object's input was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct Invalid(String);

fn non_blank(what: &str, s: &str) -> Result<String, Invalid> {
    let s = s.trim();
    if s.is_empty() {
        Err(Invalid(format!("{what} must not be empty")))
    } else {
        Ok(s.to_string())
    }
}

fn one_word(what: &str, s: &str) -> Result<String, Invalid> {
    let s = non_blank(what, s)?;
    if s.chars().any(char::is_whitespace) {
        return Err(Invalid(format!("{what} must not contain whitespace")));
    }
    Ok(s)
}

macro_rules! display_inner {
    ($ty:ty) => {
        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
        impl $ty {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

/// A revset expression: a change id, `@`, `main`, `trunk()`, ...
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Revset(String);
display_inner!(Revset);

impl Revset {
    pub fn parse(s: &str) -> Result<Self, Invalid> {
        non_blank("revset", s).map(Self)
    }

    pub fn working_copy() -> Self {
        Self("@".into())
    }

    /// The parents of this revset.
    pub fn parents(&self) -> Self {
        Self(format!("{}-", self.0))
    }
}

/// A change id as jj prints it; a unique prefix is enough.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChangeId(String);
display_inner!(ChangeId);

impl ChangeId {
    pub fn parse(s: &str) -> Result<Self, Invalid> {
        one_word("change id", s).map(Self)
    }
}

impl From<ChangeId> for Revset {
    fn from(id: ChangeId) -> Self {
        Self(id.0)
    }
}

impl From<&ChangeId> for Revset {
    fn from(id: &ChangeId) -> Self {
        Self(id.0.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommitId(String);
display_inner!(CommitId);

impl CommitId {
    pub fn parse(s: &str) -> Result<Self, Invalid> {
        one_word("commit id", s).map(Self)
    }
}

/// A local bookmark name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BookmarkName(String);
display_inner!(BookmarkName);

impl BookmarkName {
    pub fn parse(s: &str) -> Result<Self, Invalid> {
        let s = one_word("bookmark name", s)?;
        if s.contains('@') {
            return Err(Invalid(
                "bookmark name must not contain '@' (use name@remote only for remote bookmarks)"
                    .into(),
            ));
        }
        Ok(Self(s))
    }
}

/// A remote-tracking bookmark, `name@remote`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RemoteBookmark {
    pub name: BookmarkName,
    pub remote: String,
}

impl RemoteBookmark {
    pub fn parse(s: &str) -> Result<Self, Invalid> {
        let s = one_word("remote bookmark", s)?;
        let (name, remote) = s
            .split_once('@')
            .ok_or_else(|| Invalid("remote bookmark must be written name@remote".into()))?;
        Ok(Self {
            name: BookmarkName::parse(name)?,
            remote: non_blank("remote name", remote)?,
        })
    }
}

impl fmt::Display for RemoteBookmark {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.name, self.remote)
    }
}

/// A bookmark as jj lists it: local, or tracking a remote.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BookmarkRef {
    Local(BookmarkName),
    Remote(RemoteBookmark),
}

impl BookmarkRef {
    /// `name` or `name@remote`.
    pub fn parse(s: &str) -> Result<Self, Invalid> {
        if s.contains('@') {
            RemoteBookmark::parse(s).map(Self::Remote)
        } else {
            BookmarkName::parse(s).map(Self::Local)
        }
    }

    pub fn name(&self) -> &BookmarkName {
        match self {
            Self::Local(n) => n,
            Self::Remote(r) => &r.name,
        }
    }
}

impl fmt::Display for BookmarkRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local(n) => n.fmt(f),
            Self::Remote(r) => r.fmt(f),
        }
    }
}

/// A path inside the workspace, as jj reports it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepoPath(String);
display_inner!(RepoPath);

impl RepoPath {
    pub fn parse(s: &str) -> Result<Self, Invalid> {
        non_blank("path", s).map(Self)
    }
}

/// A fileset expression: a path, `glob:"*.rs"`, `~file:x`, ...
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fileset(String);
display_inner!(Fileset);

impl Fileset {
    pub fn parse(s: &str) -> Result<Self, Invalid> {
        non_blank("fileset", s).map(Self)
    }
}

/// A diff editor named in jj's `merge-tools` config.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiffTool(String);
display_inner!(DiffTool);

impl DiffTool {
    pub fn parse(s: &str) -> Result<Self, Invalid> {
        one_word("tool name", s).map(Self)
    }
}

/// An operation id from the op log.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OperationId(String);
display_inner!(OperationId);

impl OperationId {
    pub fn parse(s: &str) -> Result<Self, Invalid> {
        one_word("operation id", s).map(Self)
    }
}

/// A change description given up front (instead of opening `$EDITOR`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message(String);
display_inner!(Message);

impl Message {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

// ---- operations ------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Author {
    #[default]
    Keep,
    ResetToCurrentUser,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Describe {
    pub rev: Revset,
    pub author: Author,
}

/// Describe the working copy and start a new change on top of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub author: Author,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    ChildOf,
    After,
    Before,
}

/// Whether the working copy moves to the new change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Checkout {
    #[default]
    MoveWorkingCopy,
    StayPut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewChange {
    pub anchor: Revset,
    pub placement: Placement,
    pub checkout: Checkout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SquashSource {
    /// One working-copy file into the parent.
    WorkingCopyPath(RepoPath),
    /// A whole revision into its parent.
    Revision(Revset),
    /// The working copy into the given revision.
    WorkingCopyInto(Revset),
}

/// What happens to the source once its changes have moved out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EmptiedSource {
    #[default]
    Abandon,
    Keep,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SquashDescription {
    #[default]
    Combine,
    UseDestination,
}

/// The squash choices that do not depend on the source: what becomes of
/// the emptied source and where the result's description comes from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SquashOptions {
    pub emptied: EmptiedSource,
    pub description: SquashDescription,
}

/// How a headless squash describes the result. Combining descriptions in
/// `$EDITOR` is `Jj::squash_in_editor` instead, so neither can collide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SquashMessage {
    Given(Message),
    UseDestination,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Squash {
    pub source: SquashSource,
    pub emptied: EmptiedSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebaseKind {
    /// Just this revision; its descendants stay where they are.
    Revision,
    /// This revision and everything below it.
    WithDescendants,
    /// The whole branch the revision is on.
    Branch,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EmptiedCommits {
    #[default]
    Keep,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rebase {
    pub kind: RebaseKind,
    pub rev: Revset,
    pub onto: Revset,
    pub emptied: EmptiedCommits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookmarkOp {
    Create {
        name: BookmarkName,
        at: Revset,
    },
    Set {
        name: BookmarkName,
        at: Revset,
    },
    Delete(BookmarkName),
    Rename {
        from: BookmarkName,
        to: BookmarkName,
    },
    Forget(BookmarkName),
    Track(RemoteBookmark),
    Untrack(RemoteBookmark),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushScope {
    /// Every tracked bookmark.
    Tracked,
    Bookmark(BookmarkName),
    /// The change, under an auto-created bookmark.
    Change(Revset),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Deletions {
    #[default]
    Skip,
    Propagate,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RunMode {
    #[default]
    Apply,
    DryRun,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PushOptions {
    pub deletions: Deletions,
    pub mode: RunMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Push {
    pub scope: PushScope,
    pub options: PushOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fetch {
    DefaultRemote,
    AllRemotes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Restore {
    /// Discard one working-copy file's changes.
    Path(RepoPath),
    /// Discard every working-copy change.
    WorkingCopy,
    /// Restore paths between revisions, or undo one revision's changes.
    Spec(RestoreSpec),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreSource {
    /// Make `into` match `from`. `None` means `@-` for `from`, `@` for `into`.
    Between {
        from: Option<Revset>,
        into: Option<Revset>,
    },
    /// Undo the changes this revision made relative to its parents.
    ChangesIn(Revset),
}

/// How descendants of the restored revision are rewritten.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RestoreDescendants {
    /// Rebase them, keeping their diffs.
    #[default]
    KeepDiffs,
    /// Keep their contents, so their diffs absorb the restore.
    KeepContents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreSpec {
    pub source: RestoreSource,
    /// `None` restores every path.
    pub fileset: Option<Fileset>,
    pub descendants: RestoreDescendants,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogScope {
    /// jj's configured default revset.
    Default,
    Revset(Revset),
    /// How one revision evolved (`evolog`).
    Evolution(Revset),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogQuery {
    pub scope: LogScope,
    pub limit: Option<NonZeroUsize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revset_rejects_blank() {
        assert!(Revset::parse("  ").is_err());
        assert_eq!(Revset::parse(" @- ").unwrap().as_str(), "@-");
        assert_eq!(Revset::working_copy().parents().as_str(), "@-");
    }

    #[test]
    fn bookmark_name_rejects_remote_syntax_and_spaces() {
        assert!(BookmarkName::parse("main@origin").is_err());
        assert!(BookmarkName::parse("my branch").is_err());
        assert!(BookmarkName::parse("").is_err());
        assert_eq!(BookmarkName::parse("main").unwrap().as_str(), "main");
    }

    #[test]
    fn bookmark_ref_picks_local_or_remote() {
        assert_eq!(
            BookmarkRef::parse("main").unwrap(),
            BookmarkRef::Local(BookmarkName::parse("main").unwrap())
        );
        assert_eq!(
            BookmarkRef::parse("main@origin").unwrap().name().as_str(),
            "main"
        );
        assert!(BookmarkRef::parse("").is_err());
    }

    #[test]
    fn remote_bookmark_splits_name_and_remote() {
        let b = RemoteBookmark::parse("main@origin").unwrap();
        assert_eq!(b.name.as_str(), "main");
        assert_eq!(b.remote, "origin");
        assert_eq!(b.to_string(), "main@origin");
        assert!(RemoteBookmark::parse("main").is_err());
        assert!(RemoteBookmark::parse("main@").is_err());
        assert!(RemoteBookmark::parse("@origin").is_err());
    }
}
