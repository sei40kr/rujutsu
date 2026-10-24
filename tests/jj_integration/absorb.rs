use rujutsu::jj::Jj;

use crate::common::Repo;

#[test]
fn absorb_moves_edits_into_the_change_that_last_touched_them() {
    let repo = Repo::init()
        .file("x", "1\n2\n3\n")
        .describe("adds x")
        .bookmark("adds-x")
        .new_change("adds y")
        .file("y", "y\n")
        .new_change("wip")
        .file("x", "1\nTWO\n3\n");

    let out = repo.cli().absorb().unwrap();

    assert!(out.ok(), "{}", out.output);
    assert_eq!(repo.show("@", "empty"), "true");
    assert_eq!(repo.changed_paths("adds-x"), ["x"]);
    assert_eq!(repo.read("x"), "1\nTWO\n3\n");
}
