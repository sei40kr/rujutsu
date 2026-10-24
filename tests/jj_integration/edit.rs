use rujutsu::jj::Jj;

use crate::common::{rev, Repo};

#[test]
fn edit_makes_the_revision_the_working_copy() {
    let repo = Repo::init().describe("a").new_change("b");

    let out = repo.cli().edit(&rev("@-")).unwrap();

    assert!(out.ok(), "{}", out.output);
    assert_eq!(repo.show("@", "description.first_line()"), "a");
}
