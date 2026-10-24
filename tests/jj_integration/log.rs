use std::num::NonZeroUsize;

use rujutsu::jj::op::{LogQuery, LogScope};
use rujutsu::jj::types::LogRow;
use rujutsu::jj::Jj;

use crate::common::{rev, Repo};

fn revs(rows: &[LogRow]) -> Vec<&rujutsu::jj::types::RevEntry> {
    rows.iter()
        .filter_map(|r| match r {
            LogRow::Rev(e) => Some(e),
            LogRow::Graph(_) => None,
        })
        .collect()
}

#[test]
fn revset_scope_lists_its_changes_with_bookmarks() {
    let repo = Repo::init()
        .describe("first change")
        .bookmark("main")
        .new_change("second change");

    let rows = repo
        .cli()
        .log(&LogQuery {
            scope: LogScope::Revset(rev("::")),
            limit: None,
        })
        .unwrap();

    let revs = revs(&rows);
    assert_eq!(revs.len(), 3); // two changes + root
    assert!(revs
        .iter()
        .any(|e| e.bookmarks.iter().any(|b| b.to_string() == "main")));
}

#[test]
fn limit_caps_the_rows() {
    let repo = Repo::init()
        .describe("first change")
        .new_change("second change");

    let rows = repo
        .cli()
        .log(&LogQuery {
            scope: LogScope::Revset(rev("::")),
            limit: NonZeroUsize::new(1),
        })
        .unwrap();

    assert_eq!(revs(&rows).len(), 1);
}

#[test]
fn default_scope_uses_jjs_own_revset() {
    let repo = Repo::init();
    let rows = repo
        .cli()
        .log(&LogQuery {
            scope: LogScope::Default,
            limit: None,
        })
        .unwrap();
    assert!(revs(&rows).iter().any(|e| e.is_working_copy));
}

#[test]
fn evolution_scope_parses_through_the_log_parser() {
    let repo = Repo::init();
    let rows = repo
        .cli()
        .log(&LogQuery {
            scope: LogScope::Evolution(rev("@")),
            limit: None,
        })
        .unwrap();
    assert!(revs(&rows).iter().any(|e| e.is_working_copy));
}
