use rujutsu::jj::op::{Author, Describe};
use rujutsu::jj::Jj;

use crate::common::{msg, rev, Repo};

#[test]
fn describe_sets_the_subject() {
    let repo = Repo::init().describe("old subject");

    let out = repo
        .cli()
        .describe(
            &Describe {
                rev: rev("@"),
                author: Author::Keep,
            },
            &msg("new subject"),
        )
        .unwrap();

    assert!(out.ok(), "{}", out.output);
    assert_eq!(repo.show("@", "description.first_line()"), "new subject");
}

#[test]
fn reset_author_replaces_a_foreign_author() {
    let repo =
        Repo::init()
            .describe("x")
            .jj(&["metaedit", "--author", "Someone Else <else@example.com>"]);
    assert_eq!(repo.show("@", "author.name()"), "Someone Else");

    let out = repo
        .cli()
        .describe(
            &Describe {
                rev: rev("@"),
                author: Author::ResetToCurrentUser,
            },
            &msg("x"),
        )
        .unwrap();

    assert!(out.ok(), "{}", out.output);
    assert_eq!(repo.show("@", "author.name()"), "Test User");
}
