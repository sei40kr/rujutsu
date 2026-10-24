//! Background plumbing: jj mutations and reads on worker threads, refresh
//! generations, and process-log bookkeeping.

use std::thread;

use crate::jj::op::{LogQuery, OperationId, Revset};
use crate::jj::types::{LogRow, OpLogRow, RevisionView, StatusSnapshot};
use crate::jj::{Jj, JjError, Outcome};
use crate::keymap::PaneKind;
use crate::ui::build;
use crate::ui::pane::Pane;

use super::{App, AppEvent};

/// Entries kept in the `$` process log. Each keeps its command's full
/// output, so an unbounded log would grow for the whole session.
const PROCESS_LOG_MAX: usize = 200;

impl App {
    /// Run a jj mutation on a worker thread; completion triggers a refresh.
    pub(super) fn run_bg(
        &mut self,
        desc: impl Into<String>,
        op: impl FnOnce(&dyn Jj) -> Result<Outcome, JjError> + Send + 'static,
    ) {
        let desc = desc.into();
        self.busy = Some(desc.clone());
        let jj = self.jj.clone();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let outcome = op(&*jj).unwrap_or_else(|e| Outcome {
                cmd: "jj".into(),
                status: -1,
                output: e.to_string(),
            });
            let _ = tx.send(AppEvent::JjDone { desc, outcome });
        });
    }

    /// Refresh whatever the active buffer shows: re-run the query behind a
    /// log/revision/op-log pane, otherwise re-read the status snapshot.
    pub(super) fn refresh_current(&mut self) {
        if let Some(pane) = self.panes.last() {
            match pane.kind {
                PaneKind::Log => {
                    if let Some(query) = pane.log_query.clone() {
                        let title = pane.title.clone();
                        self.load_log(title, query, true);
                        return;
                    }
                }
                PaneKind::Revision => {
                    if let Some(rev) = pane.rev.clone() {
                        self.load_revision(rev, true);
                        return;
                    }
                }
                PaneKind::OpLog => {
                    self.load_op_log(true);
                    return;
                }
                _ => {}
            }
        }
        self.refresh();
    }

    /// Read a log-like buffer on a worker thread. The query is stored on the
    /// pane so `g` reproduces it. `replace` refreshes the current log pane
    /// instead of opening a new one.
    pub(super) fn load_log(&mut self, title: String, query: LogQuery, replace: bool) {
        self.busy = Some(title.clone());
        let jj = self.jj.clone();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let rows = jj.log(&query).unwrap_or_default();
            let _ = tx.send(AppEvent::LogReady {
                title,
                query,
                rows,
                replace,
            });
        });
    }

    /// Read one revision's header and diff for a revision buffer.
    pub(super) fn load_revision(&mut self, rev: Revset, replace: bool) {
        self.busy = Some(format!("loading {rev}"));
        let jj = self.jj.clone();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let view = jj.revision(&rev).unwrap_or_default();
            let _ = tx.send(AppEvent::RevisionReady { rev, view, replace });
        });
    }

    pub(super) fn load_op_log(&mut self, replace: bool) {
        self.busy = Some("loading op log".into());
        let jj = self.jj.clone();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let rows = jj.op_log().unwrap_or_default();
            let _ = tx.send(AppEvent::OpLogReady { rows, replace });
        });
    }

    pub(super) fn load_op_show(&mut self, op: OperationId) {
        self.busy = Some(format!("loading op {op}"));
        let jj = self.jj.clone();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let text = jj.op_show(&op).unwrap_or_default();
            let _ = tx.send(AppEvent::OpShowReady { op, text });
        });
    }

    /// Kick off a background status snapshot read.
    ///
    /// Single-flight: while a read is in flight, further requests only mark
    /// the state dirty and one follow-up read runs when it completes.
    /// Without this, watcher events arriving faster than a scan completes
    /// pile up concurrent scans that contend on the repo lock.
    pub fn refresh(&mut self) {
        if self.refresh_inflight {
            self.refresh_dirty = true;
            return;
        }
        self.refresh_inflight = true;
        self.refresh_gen += 1;
        let gen = self.refresh_gen;
        let jj = self.jj.clone();
        let log_revset = self.log_revset.clone();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = jj.snapshot(log_revset.as_ref()).map_err(|e| e.to_string());
            let _ = tx.send(AppEvent::SnapshotReady { gen, result });
        });
    }

    /// A background jj mutation finished: record it and refresh.
    pub(super) fn on_jj_done(&mut self, desc: String, outcome: Outcome) {
        self.busy = None;
        if outcome.status != 0 {
            let first = outcome
                .output
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("")
                .to_string();
            self.message = Some(format!("{desc} failed: {first}"));
        } else {
            self.message = Some(format!("{desc} done"));
        }
        self.push_process_entry(outcome);
        self.refresh_process_log_pane();
        self.refresh_current_and_status();
    }

    /// After a mutation, refresh the status snapshot and — when a query pane
    /// is on top — its query too, so log/revision buffers don't go stale.
    fn refresh_current_and_status(&mut self) {
        let top_is_query = self
            .panes
            .last()
            .is_some_and(|p| p.kind != PaneKind::Status);
        if top_is_query {
            self.refresh_current();
        }
        self.refresh();
    }

    /// `jj log` data arrived: open a log buffer, or refresh the current one.
    pub(super) fn on_log_ready(
        &mut self,
        title: String,
        query: LogQuery,
        rows: Vec<LogRow>,
        replace: bool,
    ) {
        self.busy = None;
        let root = build::build_log(&self.theme, &title, &rows);
        let top_is_log = self.panes.last().map(|p| p.kind) == Some(PaneKind::Log);
        if replace && top_is_log {
            if let Some(pane) = self.panes.last_mut() {
                pane.title = title;
                pane.log_query = Some(query);
                pane.replace_tree(root);
            }
        } else {
            let mut pane = Pane::new(PaneKind::Log, title, root);
            pane.log_query = Some(query);
            self.panes.push(pane);
        }
    }

    /// Revision data arrived: open a revision buffer, or refresh the top one.
    pub(super) fn on_revision_ready(&mut self, rev: Revset, view: RevisionView, replace: bool) {
        self.busy = None;
        let root = build::build_revision(&self.theme, &view.header, &view.files);
        let top_is_rev = self.panes.last().map(|p| p.kind) == Some(PaneKind::Revision);
        if replace && top_is_rev {
            if let Some(pane) = self.panes.last_mut() {
                pane.title = rev.to_string();
                pane.rev = Some(rev);
                pane.committed = view.files;
                pane.replace_tree(root);
            }
        } else {
            let mut pane = Pane::new(PaneKind::Revision, rev.to_string(), root);
            pane.rev = Some(rev);
            pane.committed = view.files;
            self.panes.push(pane);
        }
    }

    pub(super) fn on_op_log_ready(&mut self, rows: Vec<OpLogRow>, replace: bool) {
        self.busy = None;
        let root = build::build_op_log(&self.theme, &rows);
        let top_is_oplog = self.panes.last().map(|p| p.kind) == Some(PaneKind::OpLog);
        if replace && top_is_oplog {
            if let Some(pane) = self.panes.last_mut() {
                pane.replace_tree(root);
            }
        } else {
            self.panes
                .push(Pane::new(PaneKind::OpLog, "operation log".into(), root));
        }
    }

    pub(super) fn on_op_show_ready(&mut self, op: OperationId, text: String) {
        self.busy = None;
        let title = format!("operation {op}");
        let root = build::build_text(&title, &self.theme, &text);
        self.panes.push(Pane::new(PaneKind::OpShow, title, root));
    }

    pub(super) fn on_snapshot(&mut self, gen: u64, result: Result<StatusSnapshot, String>) {
        self.refresh_inflight = false;
        // The gen guard stays as a safety net, though single-flight means a
        // stale snapshot can no longer arrive.
        if gen == self.refresh_gen {
            match result {
                Ok(snapshot) => {
                    let root = build::build_status(&self.theme, &snapshot);
                    if let Some(pane) = self.panes.iter_mut().find(|p| p.kind == PaneKind::Status) {
                        pane.replace_tree(root);
                        pane.wc_diff = snapshot.diff.clone();
                    }
                    self.snapshot = Some(snapshot);
                }
                Err(e) => self.message = Some(format!("refresh failed: {e}")),
            }
        }
        // A refresh was requested while this one ran; run the follow-up now.
        if self.refresh_dirty {
            self.refresh_dirty = false;
            self.refresh();
        }
    }

    /// The editor ran in the foreground; record the result and refresh.
    pub fn on_editor_done(&mut self, desc: String, outcome: Result<Outcome, JjError>) {
        let outcome = outcome.unwrap_or_else(|e| Outcome {
            cmd: "jj".into(),
            status: -1,
            output: e.to_string(),
        });
        self.message = Some(if outcome.ok() {
            format!("{desc} done")
        } else {
            format!("{desc} exited with {}", outcome.status)
        });
        self.push_process_entry(outcome);
        self.refresh_process_log_pane();
        self.refresh_current_and_status();
    }

    pub(super) fn open_op_log(&mut self) {
        if self.panes.last().map(|p| p.kind) == Some(PaneKind::OpLog) {
            return;
        }
        self.load_op_log(false);
    }

    pub(super) fn open_process_log(&mut self) {
        if self.panes.last().map(|p| p.kind) == Some(PaneKind::ProcessLog) {
            return;
        }
        let root =
            build::build_process_log(&self.theme, &self.process_log, self.process_log_dropped);
        self.panes.push(Pane::new(
            PaneKind::ProcessLog,
            "jj process log".into(),
            root,
        ));
    }

    pub(super) fn refresh_process_log_pane(&mut self) {
        if let Some(pane) = self
            .panes
            .iter_mut()
            .find(|p| p.kind == PaneKind::ProcessLog)
        {
            pane.replace_tree(build::build_process_log(
                &self.theme,
                &self.process_log,
                self.process_log_dropped,
            ));
        }
    }

    fn push_process_entry(&mut self, entry: Outcome) {
        self.process_log.push(entry);
        if self.process_log.len() > PROCESS_LOG_MAX {
            let excess = self.process_log.len() - PROCESS_LOG_MAX;
            self.process_log.drain(..excess);
            self.process_log_dropped += excess;
        }
    }
}
