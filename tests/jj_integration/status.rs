use rujutsu::jj::op::BookmarkRef;
use rujutsu::jj::types::LogRow;
use rujutsu::jj::Jj;

use crate::common::{rev, Repo};

#[test]
fn snapshot_reads_working_copy_diff_log_and_bookmarks() {
    let repo = Repo::init()
        .file("a.txt", "hello\n")
        .describe("first change")
        .bookmark("main")
        .new_change("second change")
        .file("a.txt", "hello\nline2\n")
        .file("b.txt", "new\n");

    let snapshot = repo.cli().snapshot(None).unwrap();

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
        .any(|b| matches!(b, BookmarkRef::Local(n) if n.as_str() == "main")));
    assert!(snapshot.conflicts.is_empty());
}

#[test]
fn snapshot_log_section_follows_the_configured_revset() {
    let repo = Repo::init()
        .describe("first change")
        .new_change("second change");

    let snapshot = repo.cli().snapshot(Some(&rev("@"))).unwrap();

    let revs = snapshot
        .log
        .iter()
        .filter(|r| matches!(r, LogRow::Rev(_)))
        .count();
    assert_eq!(revs, 1);
}
