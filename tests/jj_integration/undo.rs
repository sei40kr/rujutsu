use rujutsu::jj::Jj;

use crate::common::Repo;

#[test]
fn undo_then_redo_walk_the_operation_log() {
    let repo = Repo::init().describe("before").describe("after");
    let cli = repo.cli();

    let out = cli.undo().unwrap();
    assert!(out.ok(), "{}", out.output);
    assert_eq!(repo.show("@", "description.first_line()"), "before");

    let out = cli.redo().unwrap();
    assert!(out.ok(), "{}", out.output);
    assert_eq!(repo.show("@", "description.first_line()"), "after");
}
