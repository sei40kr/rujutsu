use rujutsu::jj::op::{Author, Commit};
use rujutsu::jj::Jj;

use crate::common::{msg, Repo};

#[test]
fn commit_describes_the_working_copy_and_starts_a_new_change() {
    let repo = Repo::init().file("a.txt", "hello\n");

    let out = repo
        .cli()
        .commit(
            &Commit {
                author: Author::Keep,
            },
            &msg("add a"),
        )
        .unwrap();

    assert!(out.ok(), "{}", out.output);
    assert_eq!(repo.show("@-", "description.first_line()"), "add a");
    assert_eq!(repo.show("@", "empty"), "true");
}
