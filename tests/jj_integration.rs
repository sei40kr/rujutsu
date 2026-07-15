//! Integration tests that exercise `JjClient` against a real `jj` binary.
//! Skipped (silently passing) when `jj` is not installed.

use std::fs;
use std::path::Path;
use std::process::Command;

use rujutsu::jj::client::JjClient;
use rujutsu::jj::types::LogRow;

fn jj_available() -> bool {
    Command::new("jj")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn jj(dir: &Path, args: &[&str]) {
    let out = Command::new("jj")
        .args(args)
        .current_dir(dir)
        .env("JJ_USER", "Test User")
        .env("JJ_EMAIL", "test@example.com")
        .output()
        .expect("failed to run jj");
    assert!(
        out.status.success(),
        "jj {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A repo with one described, bookmarked commit and a dirty working copy.
fn setup_repo(dir: &Path) {
    jj(dir, &["git", "init"]);
    fs::write(dir.join("a.txt"), "hello\n").unwrap();
    jj(dir, &["describe", "-m", "first change"]);
    jj(dir, &["bookmark", "create", "main", "-r", "@"]);
    jj(dir, &["new", "-m", "second change"]);
    fs::write(dir.join("a.txt"), "hello\nline2\n").unwrap();
    fs::write(dir.join("b.txt"), "new\n").unwrap();
}

#[test]
fn snapshot_reads_working_copy_diff_log_and_bookmarks() {
    if !jj_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    setup_repo(tmp.path());
    let client = JjClient::discover(tmp.path()).unwrap();

    let snapshot = client.read_snapshot().unwrap();

    let wc = snapshot.working_copy.expect("working copy entry");
    assert!(wc.is_working_copy);
    assert_eq!(wc.subject, "second change");

    assert_eq!(snapshot.parents.len(), 1);
    assert_eq!(snapshot.parents[0].subject, "first change");

    // The snapshot picked up the external file edits.
    let paths: Vec<&str> = snapshot.diff.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, vec!["a.txt", "b.txt"]);
    assert!(snapshot.diff[1].is_new);
    assert!(!snapshot.diff[0].hunks.is_empty());

    // Log contains both changes plus the root; bookmarks list has main.
    let subjects: Vec<String> = snapshot
        .log
        .iter()
        .filter_map(|r| match r {
            LogRow::Rev(e) => Some(e.subject.clone()),
            LogRow::Graph(_) => None,
        })
        .collect();
    assert!(subjects.contains(&"first change".to_string()));
    assert!(subjects.contains(&"second change".to_string()));
    assert!(snapshot
        .bookmarks
        .iter()
        .any(|b| b.name == "main" && b.remote.is_none()));
    assert!(snapshot.conflicts.is_empty());
}

#[test]
fn log_revision_and_op_log_reads_parse() {
    if !jj_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    setup_repo(tmp.path());
    let client = JjClient::discover(tmp.path()).unwrap();
    // Snapshot the working copy first, as the app does on startup.
    let snapshot = client.read_snapshot().unwrap();

    let rows = client
        .read_log(&["log".into(), "-r".into(), "::".into()])
        .unwrap();
    let revs: Vec<_> = rows
        .iter()
        .filter(|r| matches!(r, LogRow::Rev(_)))
        .collect();
    assert_eq!(revs.len(), 3); // two changes + root

    // The bookmarked parent renders its bookmark and the root is flagged.
    let bookmark_row = rows.iter().find_map(|r| match r {
        LogRow::Rev(e) if e.bookmarks.contains("main") => Some(e),
        _ => None,
    });
    assert!(bookmark_row.is_some());

    let parent = &snapshot.parents[0];
    let (header, diff) = client.read_revision(&parent.change_id).unwrap();
    assert!(header.contains("first change"));
    assert!(diff.contains("+hello"));

    let ops = client.read_op_log().unwrap();
    assert!(ops
        .iter()
        .any(|r| matches!(r, rujutsu::jj::types::OpLogRow::Op(o) if o.is_current)));

    // Evolog of @ parses through the same row parser.
    let evolog = client
        .read_log(&["evolog".into(), "-r".into(), "@".into()])
        .unwrap();
    assert!(evolog
        .iter()
        .any(|r| matches!(r, LogRow::Rev(e) if e.is_working_copy)));
}

#[test]
fn mutations_run_and_reads_reflect_them() {
    if !jj_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    setup_repo(tmp.path());
    let client = JjClient::discover(tmp.path()).unwrap();

    // Describe the working copy without an editor.
    let out = client
        .run(&["describe", "-m", "renamed subject", "@"])
        .unwrap();
    assert!(out.ok(), "describe failed: {}", out.stderr);

    let snapshot = client.read_snapshot().unwrap();
    assert_eq!(snapshot.working_copy.unwrap().subject, "renamed subject");

    // Undo restores the previous description.
    assert!(client.run(&["undo"]).unwrap().ok());
    let snapshot = client.read_snapshot().unwrap();
    assert_eq!(snapshot.working_copy.unwrap().subject, "second change");
}
