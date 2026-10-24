use rujutsu::jj::Jj;

use crate::common::{rev, Repo};

#[test]
fn abandon_drops_the_revision_and_reparents_its_child() {
    let repo = Repo::init().describe("a").new_change("b").new_change("c");

    let out = repo.cli().abandon(&rev("@-")).unwrap();

    assert!(out.ok(), "{}", out.output);
    assert_eq!(repo.subjects(), ["c", "a", ""]);
}
