use rujutsu::jj::Jj;

use crate::common::{rev, Repo};

#[test]
fn duplicate_copies_the_revision() {
    let repo = Repo::init().describe("a").new_change("b");

    let out = repo.cli().duplicate(&rev("@-")).unwrap();

    assert!(out.ok(), "{}", out.output);
    let copies = repo.subjects().iter().filter(|s| *s == "a").count();
    assert_eq!(copies, 2);
}
