use rujutsu::jj::op::{EmptiedSource, RepoPath, Squash, SquashMessage, SquashSource};
use rujutsu::jj::Jj;

use crate::common::{msg, rev, Repo};

/// `a` adds a.txt; the working copy `b` edits a.txt and adds b.txt.
fn stacked(repo: Repo) -> Repo {
    repo.file("a.txt", "one\n")
        .describe("a")
        .new_change("b")
        .file("a.txt", "one\ntwo\n")
        .file("b.txt", "new\n")
}

fn squash(repo: &Repo, op: Squash, m: SquashMessage) {
    let out = repo.cli().squash(&op, &m).unwrap();
    assert!(out.ok(), "{}", out.output);
}

#[test]
fn revision_folds_into_its_parent() {
    let repo = Repo::init();
    let repo = stacked(repo);

    squash(
        &repo,
        Squash {
            source: SquashSource::Revision(rev("@")),
            emptied: EmptiedSource::Abandon,
        },
        SquashMessage::Given(msg("merged")),
    );

    assert_eq!(repo.show("@-", "description.first_line()"), "merged");
    assert_eq!(repo.changed_paths("@-"), ["a.txt", "b.txt"]);
    assert_eq!(repo.show("@", "empty"), "true");
    assert_eq!(repo.subjects().len(), 3); // merged + new empty @ + root
}

#[test]
fn working_copy_path_moves_just_that_file() {
    let repo = Repo::init();
    let repo = stacked(repo);

    squash(
        &repo,
        Squash {
            source: SquashSource::WorkingCopyPath(RepoPath::parse("b.txt").unwrap()),
            emptied: EmptiedSource::Abandon,
        },
        SquashMessage::Given(msg("a with b")),
    );

    assert_eq!(repo.changed_paths("@-"), ["a.txt", "b.txt"]);
    assert_eq!(repo.changed_paths("@"), ["a.txt"]);
    assert_eq!(repo.show("@", "description.first_line()"), "b");
}

#[test]
fn working_copy_into_targets_the_given_revision() {
    let repo = Repo::init();
    let repo = stacked(repo);

    squash(
        &repo,
        Squash {
            source: SquashSource::WorkingCopyInto(rev("@-")),
            emptied: EmptiedSource::Abandon,
        },
        SquashMessage::UseDestination,
    );

    assert_eq!(repo.show("@-", "description.first_line()"), "a");
    assert_eq!(repo.changed_paths("@-"), ["a.txt", "b.txt"]);
}

#[test]
fn keep_emptied_leaves_the_source_in_place() {
    let repo = Repo::init();
    let repo = stacked(repo);

    squash(
        &repo,
        Squash {
            source: SquashSource::Revision(rev("@")),
            emptied: EmptiedSource::Keep,
        },
        SquashMessage::Given(msg("merged")),
    );

    assert_eq!(repo.show("@", "empty"), "true");
    assert_eq!(repo.show("@", "description.first_line()"), "b");
    assert_eq!(repo.subjects().len(), 3); // merged + kept b + root
}
