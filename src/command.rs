//! Commands are data, not closures: keymaps bind keys to `Command` values,
//! config remaps by name, and the help UI enumerates `COMMANDS`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    Quit,
    Refresh,
    Nav(NavCmd),
    Fold(FoldCmd),
    /// Show the thing at point (revision, operation).
    Visit,
    /// Abandon the revision at point, or restore (discard) the file at point.
    AbandonOrRestore,
    /// `jj edit` the revision at point.
    Edit,
    /// `jj absorb` the working copy into mutable ancestors.
    Absorb,
    /// `jj duplicate` the revision at point.
    Duplicate,
    /// `jj split` the revision at point (interactive diff editor).
    Split,
    /// `jj evolog` of the revision at point.
    Evolog,
    /// `jj undo` / `jj redo`.
    Undo,
    Redo,
    Search,
    Transient(Menu),
    /// `jj op restore` to the operation at point (op-log buffer only).
    OpRestore,
    Help,
    OpLog,
    ProcessLog,
    /// Copy the value at point (revision/operation/file) to the clipboard.
    Copy,
    /// Copy the revision the current buffer is about to the clipboard.
    CopyRevision,
}

/// Pure cursor motions. Grouped so `dispatch` forwards them wholesale to
/// `Pane::navigate` instead of growing one arm per motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavCmd {
    MoveDown,
    MoveUp,
    HalfPageDown,
    HalfPageUp,
    GotoTop,
    GotoBottom,
    NextSection,
    PrevSection,
    ParentSection,
}

/// Fold operations (vim's `z` family). Grouped like `NavCmd` so `dispatch`
/// forwards them wholesale to `Pane::fold` instead of growing one arm per
/// operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FoldCmd {
    /// `za` — toggle the fold at point.
    Toggle,
    /// `zA` — toggle the fold at point, applying the new state recursively.
    ToggleRec,
    /// `zo` — open the fold at point.
    Open,
    /// `zO` — open the fold at point and all folds inside it.
    OpenRec,
    /// `zc` — close the fold at point (the enclosing one when already closed).
    Close,
    /// `zC` — close the fold at point and all folds inside it.
    CloseRec,
    /// `zR` — open every fold in the buffer.
    OpenAll,
    /// `zM` — close every fold in the buffer.
    CloseAll,
    /// `zr` — open the shallowest closed fold level.
    OpenLevel,
    /// `zm` — close the deepest open fold level.
    CloseLevel,
}

/// Transient menus. `ui::transient::menu_def` maps each to its definition,
/// so opening a new menu never adds a `dispatch` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Menu {
    Describe,
    Commit,
    New,
    Squash,
    Rebase,
    Bookmark,
    Push,
    Fetch,
    Log,
}

pub struct CommandInfo {
    pub cmd: Command,
    pub name: &'static str,
    pub desc: &'static str,
}

pub const COMMANDS: &[CommandInfo] = &[
    ci(Command::Quit, "quit", "Close current buffer (quit if last)"),
    ci(Command::Refresh, "refresh", "Refresh the current buffer"),
    ci(
        Command::Nav(NavCmd::MoveDown),
        "move-down",
        "Move cursor down",
    ),
    ci(Command::Nav(NavCmd::MoveUp), "move-up", "Move cursor up"),
    ci(
        Command::Nav(NavCmd::HalfPageDown),
        "half-page-down",
        "Scroll half a page down",
    ),
    ci(
        Command::Nav(NavCmd::HalfPageUp),
        "half-page-up",
        "Scroll half a page up",
    ),
    ci(
        Command::Nav(NavCmd::GotoTop),
        "goto-top",
        "Go to the first line",
    ),
    ci(
        Command::Nav(NavCmd::GotoBottom),
        "goto-bottom",
        "Go to the last line",
    ),
    ci(
        Command::Nav(NavCmd::NextSection),
        "next-section",
        "Jump to next section heading",
    ),
    ci(
        Command::Nav(NavCmd::PrevSection),
        "prev-section",
        "Jump to previous section heading",
    ),
    ci(
        Command::Nav(NavCmd::ParentSection),
        "parent-section",
        "Jump to parent section",
    ),
    ci(
        Command::Fold(FoldCmd::Toggle),
        "toggle-section",
        "Collapse/expand section at point",
    ),
    ci(
        Command::Fold(FoldCmd::ToggleRec),
        "fold-toggle-rec",
        "Toggle the fold at point recursively",
    ),
    ci(
        Command::Fold(FoldCmd::Open),
        "fold-open",
        "Open the fold at point",
    ),
    ci(
        Command::Fold(FoldCmd::OpenRec),
        "fold-open-rec",
        "Open the fold at point recursively",
    ),
    ci(
        Command::Fold(FoldCmd::Close),
        "fold-close",
        "Close the fold at point (or its parent)",
    ),
    ci(
        Command::Fold(FoldCmd::CloseRec),
        "fold-close-rec",
        "Close the fold at point recursively",
    ),
    ci(
        Command::Fold(FoldCmd::OpenAll),
        "fold-open-all",
        "Open all folds",
    ),
    ci(
        Command::Fold(FoldCmd::CloseAll),
        "fold-close-all",
        "Close all folds",
    ),
    ci(
        Command::Fold(FoldCmd::OpenLevel),
        "fold-less",
        "Open one more level of folds",
    ),
    ci(
        Command::Fold(FoldCmd::CloseLevel),
        "fold-more",
        "Close one more level of folds",
    ),
    ci(
        Command::Visit,
        "visit",
        "Show the thing at point (revision/operation)",
    ),
    ci(
        Command::AbandonOrRestore,
        "abandon-or-restore",
        "Abandon revision or restore file at point",
    ),
    ci(
        Command::Edit,
        "edit",
        "Edit the revision at point (jj edit)",
    ),
    ci(
        Command::Absorb,
        "absorb",
        "Absorb working-copy changes into mutable ancestors",
    ),
    ci(
        Command::Duplicate,
        "duplicate",
        "Duplicate the revision at point",
    ),
    ci(
        Command::Split,
        "split",
        "Split the revision at point interactively",
    ),
    ci(
        Command::Evolog,
        "evolog",
        "Show how the revision at point evolved",
    ),
    ci(Command::Undo, "undo", "Undo the last operation (jj undo)"),
    ci(
        Command::Redo,
        "redo",
        "Redo the most recently undone operation",
    ),
    ci(
        Command::Search,
        "search",
        "Incremental search in the buffer",
    ),
    ci(
        Command::Transient(Menu::Describe),
        "describe",
        "Open the describe menu",
    ),
    ci(
        Command::Transient(Menu::Commit),
        "commit",
        "Open the commit menu",
    ),
    ci(Command::Transient(Menu::New), "new", "Open the new menu"),
    ci(
        Command::Transient(Menu::Squash),
        "squash",
        "Open the squash menu",
    ),
    ci(
        Command::Transient(Menu::Rebase),
        "rebase",
        "Open the rebase menu",
    ),
    ci(
        Command::Transient(Menu::Bookmark),
        "bookmark",
        "Open the bookmark menu",
    ),
    ci(Command::Transient(Menu::Push), "push", "Open the push menu"),
    ci(
        Command::Transient(Menu::Fetch),
        "fetch",
        "Open the fetch menu",
    ),
    ci(Command::Transient(Menu::Log), "log", "Open the log menu"),
    ci(
        Command::OpRestore,
        "op-restore",
        "Restore the repo to the operation at point",
    ),
    ci(Command::Help, "help", "Show key bindings"),
    ci(Command::OpLog, "op-log", "Show the operation log"),
    ci(
        Command::ProcessLog,
        "process-log",
        "Show the jj process log",
    ),
    ci(
        Command::Copy,
        "copy",
        "Copy the value at point to the clipboard",
    ),
    ci(
        Command::CopyRevision,
        "copy-revision",
        "Copy the buffer's revision to the clipboard",
    ),
];

const fn ci(cmd: Command, name: &'static str, desc: &'static str) -> CommandInfo {
    CommandInfo { cmd, name, desc }
}

pub fn by_name(name: &str) -> Option<Command> {
    COMMANDS.iter().find(|c| c.name == name).map(|c| c.cmd)
}

pub fn info(cmd: Command) -> &'static CommandInfo {
    COMMANDS
        .iter()
        .find(|c| c.cmd == cmd)
        .expect("every Command has a COMMANDS entry")
}
