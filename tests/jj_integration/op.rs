use rujutsu::jj::types::OpLogRow;
use rujutsu::jj::Jj;

use crate::common::Repo;

fn ops(repo: &Repo) -> Vec<rujutsu::jj::types::OpEntry> {
    repo.cli()
        .op_log()
        .unwrap()
        .into_iter()
        .filter_map(|r| match r {
            OpLogRow::Op(o) => Some(o),
            OpLogRow::Graph(_) => None,
        })
        .collect()
}

#[test]
fn op_log_marks_the_current_operation() {
    let repo = Repo::init();
    let ops = ops(&repo);
    assert!(ops.iter().any(|o| o.is_current));
}

#[test]
fn op_show_describes_an_operation() {
    let repo = Repo::init().describe("first change");
    let current = ops(&repo).into_iter().find(|o| o.is_current).unwrap();

    let text = repo.cli().op_show(&current.id).unwrap();

    assert!(text.contains("describe"), "{text}");
}

#[test]
fn op_restore_rolls_the_repo_back() {
    let repo = Repo::init().describe("before");
    let target = ops(&repo).into_iter().find(|o| o.is_current).unwrap();
    let repo = repo.describe("after");

    let out = repo.cli().op_restore(&target.id).unwrap();

    assert!(out.ok(), "{}", out.output);
    assert_eq!(repo.show("@", "description.first_line()"), "before");
}
