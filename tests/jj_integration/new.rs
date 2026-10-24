use rujutsu::jj::op::{Checkout, NewChange, Placement};
use rujutsu::jj::Jj;

use crate::common::{rev, Repo};

/// `a` with a child `b` that is the working copy.
fn two(repo: Repo) -> Repo {
    repo.describe("a").new_change("b")
}

fn new_change(repo: &Repo, op: NewChange) {
    let out = repo.cli().new_change(&op).unwrap();
    assert!(out.ok(), "{}", out.output);
}

#[test]
fn child_of_moves_the_working_copy_onto_the_anchor() {
    let repo = Repo::init();
    let repo = two(repo);

    new_change(
        &repo,
        NewChange {
            anchor: rev("@-"),
            placement: Placement::ChildOf,
            checkout: Checkout::MoveWorkingCopy,
        },
    );

    assert_eq!(repo.show("@-", "description.first_line()"), "a");
    assert_eq!(repo.show("@", "empty"), "true");
    assert_eq!(repo.subjects().len(), 4);
}

#[test]
fn after_inserts_between_the_anchor_and_its_child() {
    let repo = Repo::init();
    let repo = two(repo);

    new_change(
        &repo,
        NewChange {
            anchor: rev("@-"),
            placement: Placement::After,
            checkout: Checkout::MoveWorkingCopy,
        },
    );

    assert_eq!(repo.show("@-", "description.first_line()"), "a");
    assert_eq!(repo.show("@+", "description.first_line()"), "b");
}

#[test]
fn before_inserts_between_the_anchor_and_its_parent() {
    let repo = Repo::init();
    let repo = two(repo);

    new_change(
        &repo,
        NewChange {
            anchor: rev("@"),
            placement: Placement::Before,
            checkout: Checkout::MoveWorkingCopy,
        },
    );

    assert_eq!(repo.show("@-", "description.first_line()"), "a");
    assert_eq!(repo.show("@+", "description.first_line()"), "b");
}

#[test]
fn stay_put_leaves_the_working_copy_where_it_was() {
    let repo = Repo::init();
    let repo = two(repo);

    new_change(
        &repo,
        NewChange {
            anchor: rev("@-"),
            placement: Placement::ChildOf,
            checkout: Checkout::StayPut,
        },
    );

    assert_eq!(repo.show("@", "description.first_line()"), "b");
    assert_eq!(repo.subjects().len(), 4);
}
