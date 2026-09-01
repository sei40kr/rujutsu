//! Application state and the update half of the Elm-style loop. All jj
//! mutations run on worker threads; results come back as `AppEvent`s.
//!
//! `App`'s behavior is split across submodules by concern: key routing in
//! `keys`, cursor DWIM in `dwim`, buffer search in `search`, background
//! plumbing in `workers`, and one module per transient menu family under
//! `ops`. This file owns the state, the event loop entry (`update`) and the
//! command dispatch table.

mod dwim;
mod input;
mod keys;
mod ops;
mod search;
mod workers;

pub use search::SearchState;

use input::Input;

use crossbeam_channel::Sender;
use ratatui::crossterm::event::KeyEvent;

use crate::command::{Command, NavCmd};
use crate::jj::client::{JjClient, ProcessEntry};
use crate::jj::types::{FileDiff, LogRow, OpLogRow, StatusSnapshot};
use crate::keymap::{KeyPress, Keymaps, PaneKind};
use crate::theme::Theme;
use crate::ui::pane::Pane;
use crate::ui::transient::TransientState;

#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // events are few and short-lived
pub enum AppEvent {
    Key(KeyEvent),
    Resize,
    /// A background snapshot read finished.
    SnapshotReady {
        gen: u64,
        result: Result<StatusSnapshot, String>,
    },
    /// A background jj mutation finished.
    JjDone {
        desc: String,
        entry: ProcessEntry,
    },
    /// `jj log`/`jj evolog` data for a log buffer arrived. `replace` re-uses
    /// the current log pane (a refresh) instead of pushing a new one.
    LogReady {
        title: String,
        args: Vec<String>,
        rows: Vec<LogRow>,
        replace: bool,
    },
    /// Header + diff for a revision buffer arrived.
    RevisionReady {
        rev: String,
        header: String,
        files: Vec<FileDiff>,
        replace: bool,
    },
    /// `jj op log` data arrived.
    OpLogReady {
        rows: Vec<OpLogRow>,
        replace: bool,
    },
    /// `jj op show` output arrived.
    OpShowReady {
        op_id: String,
        text: String,
    },
    /// The fs watcher saw the op store change.
    RepoChanged,
}

/// A destructive action awaiting y/n confirmation.
pub struct Confirm {
    pub prompt: String,
    pub action: PendingAction,
}

pub enum PendingAction {
    Jj { desc: String, args: Vec<String> },
}

/// Commands like `jj describe` must run with the terminal handed over to
/// $EDITOR; the main loop performs this outside of raw mode.
pub struct EditorRequest {
    pub desc: String,
    pub args: Vec<String>,
    /// Extra environment for the jj process.
    pub envs: Vec<(String, String)>,
}

impl EditorRequest {
    pub fn new(desc: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            desc: desc.into(),
            args,
            envs: Vec::new(),
        }
    }
}

pub struct App {
    pub jj: JjClient,
    pub tx: Sender<AppEvent>,
    pub panes: Vec<Pane>,
    pub keymaps: Keymaps,
    pub theme: Theme,
    pub scrolloff: usize,
    pub pending: Vec<KeyPress>,
    pub transient: Option<TransientState>,
    pub input: Option<Input>,
    pub confirm: Option<Confirm>,
    pub show_help: bool,
    /// Scroll offset of the help overlay; clamped to the content by render.
    pub help_scroll: usize,
    pub message: Option<String>,
    pub busy: Option<String>,
    pub process_log: Vec<ProcessEntry>,
    /// How many old entries have been trimmed off `process_log`; keeps the
    /// `$` buffer's section identities stable so fold state and the cursor
    /// don't jump to a different entry after a trim.
    pub process_log_dropped: usize,
    pub snapshot: Option<StatusSnapshot>,
    pub search: SearchState,
    pub should_quit: bool,
    editor_request: Option<EditorRequest>,
    /// Text to place on the system clipboard. The main loop owns the
    /// terminal, so it performs the copy after `update`.
    clipboard_request: Option<String>,
    refresh_gen: u64,
    /// A snapshot read is in flight. Refreshes requested meanwhile only set
    /// `refresh_dirty` — the watcher can fire faster than a scan completes,
    /// and concurrent scans contend on the repo lock.
    refresh_inflight: bool,
    refresh_dirty: bool,
}

impl App {
    pub fn new(
        jj: JjClient,
        tx: Sender<AppEvent>,
        keymaps: Keymaps,
        theme: Theme,
        scrolloff: usize,
    ) -> Self {
        let title = format!(
            "rujutsu: {}",
            jj.workspace_root
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        );
        let status = Pane::new(PaneKind::Status, title, crate::ui::section::Section::root());
        Self {
            jj,
            tx,
            panes: vec![status],
            keymaps,
            theme,
            scrolloff,
            pending: Vec::new(),
            transient: None,
            input: None,
            confirm: None,
            show_help: false,
            help_scroll: 0,
            message: None,
            busy: None,
            process_log: Vec::new(),
            process_log_dropped: 0,
            snapshot: None,
            search: SearchState::default(),
            should_quit: false,
            editor_request: None,
            clipboard_request: None,
            refresh_gen: 0,
            refresh_inflight: false,
            refresh_dirty: false,
        }
    }

    pub fn take_editor_request(&mut self) -> Option<EditorRequest> {
        self.editor_request.take()
    }

    pub fn take_clipboard_request(&mut self) -> Option<String> {
        self.clipboard_request.take()
    }

    pub(crate) fn request_editor(&mut self, req: EditorRequest) {
        self.editor_request = Some(req);
    }

    pub fn which_key_candidates(&self) -> Vec<(String, String)> {
        let kind = self
            .panes
            .last()
            .map(|p| p.kind)
            .unwrap_or(PaneKind::Status);
        self.keymaps.candidates(kind, &self.pending)
    }

    // ---- event handling ----------------------------------------------------

    /// Pure router: every arm is a one-line delegation. Event bodies live in
    /// the submodule that owns the concern (see the module docs).
    pub fn update(&mut self, event: AppEvent) {
        match event {
            AppEvent::Key(ev) => self.on_key(ev),
            AppEvent::Resize => {}
            AppEvent::SnapshotReady { gen, result } => self.on_snapshot(gen, result),
            AppEvent::JjDone { desc, entry } => self.on_jj_done(desc, entry),
            AppEvent::LogReady {
                title,
                args,
                rows,
                replace,
            } => self.on_log_ready(title, args, rows, replace),
            AppEvent::RevisionReady {
                rev,
                header,
                files,
                replace,
            } => self.on_revision_ready(rev, header, files, replace),
            AppEvent::OpLogReady { rows, replace } => self.on_op_log_ready(rows, replace),
            AppEvent::OpShowReady { op_id, text } => self.on_op_show_ready(op_id, text),
            AppEvent::RepoChanged => self.refresh(),
        }
    }

    // ---- command dispatch --------------------------------------------------

    /// Pure router, like `update`: an arm that grows a body gets extracted
    /// into a submodule method.
    fn dispatch(&mut self, cmd: Command) {
        let height = 40; // page motions use a nominal height; follow() clamps
        match cmd {
            Command::Quit => self.quit_or_pop(),
            Command::Refresh => {
                self.refresh_current();
                self.message = Some("refreshing".into());
            }
            // While a search is active, n/p walk matches instead of sections.
            Command::Nav(NavCmd::NextSection) if self.search.query.is_some() => self.search_move(1),
            Command::Nav(NavCmd::PrevSection) if self.search.query.is_some() => {
                self.search_move(-1)
            }
            Command::Nav(nav) => self.pane_mut(|p| p.navigate(nav, height)),
            Command::Fold(fold) => self.pane_mut(|p| p.fold(fold)),
            Command::Visit => self.visit_at_point(),
            Command::AbandonOrRestore => self.abandon_or_restore_at_point(),
            Command::Edit => self.edit_at_point(),
            Command::Absorb => self.absorb(),
            Command::Duplicate => self.duplicate_at_point(),
            Command::Split => self.split_at_point(),
            Command::Evolog => self.evolog_at_point(),
            Command::Undo => self.run_jj_bg("undo".into(), svec(&["undo"])),
            Command::Redo => self.run_jj_bg("redo".into(), svec(&["redo"])),
            Command::Search => self.start_search(),
            Command::Transient(menu) => self.open_transient(menu),
            Command::OpRestore => self.op_restore_at_point(),
            Command::Help => {
                self.show_help = true;
                self.help_scroll = 0;
            }
            Command::OpLog => self.open_op_log(),
            Command::ProcessLog => self.open_process_log(),
            Command::Copy => self.copy_at_point(),
            Command::CopyRevision => self.copy_buffer_revision(),
        }
    }

    fn quit_or_pop(&mut self) {
        if self.panes.len() > 1 {
            self.panes.pop();
        } else {
            self.should_quit = true;
        }
    }

    fn pane_mut(&mut self, f: impl FnOnce(&mut Pane)) {
        if let Some(p) = self.panes.last_mut() {
            f(p);
        }
    }

    /// The change id of the revision at point, falling back to `@`. Most
    /// revision-targeting commands use this DWIM rule.
    pub(crate) fn rev_at_point(&self) -> String {
        use crate::ui::section::SectionValue;
        match self.panes.last().map(|p| p.value_at_cursor()) {
            Some(SectionValue::Revision { change_id }) => change_id,
            _ => "@".to_string(),
        }
    }
}

pub(crate) fn svec(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}
