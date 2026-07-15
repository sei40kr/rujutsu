//! Event sources feeding the main loop: terminal input and the `.jj`
//! directory watcher. Both forward into one `AppEvent` channel.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::Sender;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode, DebounceEventResult, Debouncer};
use ratatui::crossterm::event::{self, Event};

use crate::app::AppEvent;

/// Spawn the terminal input reader. While `paused` is set (an external
/// $EDITOR owns the terminal) the thread must not touch stdin at all.
pub fn spawn_input_thread(tx: Sender<AppEvent>, paused: Arc<AtomicBool>) {
    thread::spawn(move || {
        loop {
            if paused.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(50));
                continue;
            }
            // Poll with a timeout so a pause request takes effect promptly.
            match event::poll(Duration::from_millis(100)) {
                Ok(true) => {
                    if paused.load(Ordering::SeqCst) {
                        continue;
                    }
                    let ev = match event::read() {
                        Ok(ev) => ev,
                        Err(_) => return,
                    };
                    let msg = match ev {
                        Event::Key(k) => AppEvent::Key(k),
                        Event::Resize(_, _) => AppEvent::Resize,
                        _ => continue,
                    };
                    if tx.send(msg).is_err() {
                        return;
                    }
                }
                Ok(false) => {}
                Err(_) => return,
            }
        }
    });
}

/// Watch the repo's operation store so external `jj` invocations refresh the
/// status buffer. Every mutation writes a new head under
/// `.jj/repo/op_heads`, and — unlike the working-copy state files, which are
/// rewritten by snapshots our own reads trigger — it only changes when an
/// actual operation ran, so watching it cannot feed back into itself.
/// Returns the debouncer, which must stay alive for the watch to persist.
pub fn spawn_repo_watcher(
    tx: Sender<AppEvent>,
    jj_dir: &Path,
) -> Option<Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>> {
    let mut debouncer = new_debouncer(
        Duration::from_millis(200),
        move |res: DebounceEventResult| {
            if res.is_ok() {
                let _ = tx.send(AppEvent::RepoChanged);
            }
        },
    )
    .ok()?;
    // In a secondary workspace `.jj/repo` is a file naming the real repo
    // directory; resolve it so the watch lands on the shared op store.
    let repo = jj_dir.join("repo");
    let repo = if repo.is_file() {
        std::fs::read_to_string(&repo)
            .map(|p| std::path::PathBuf::from(p.trim()))
            .unwrap_or(repo)
    } else {
        repo
    };
    let op_heads = repo.join("op_heads");
    let target = if op_heads.is_dir() {
        op_heads
    } else {
        jj_dir.to_path_buf()
    };
    debouncer
        .watcher()
        .watch(&target, RecursiveMode::Recursive)
        .ok()?;
    Some(debouncer)
}
