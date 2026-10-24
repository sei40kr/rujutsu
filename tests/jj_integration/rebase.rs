use rujutsu::jj::op::{EmptiedCommits, Rebase, RebaseKind};
use rujutsu::jj::Jj;

use crate::common::{rev, Repo};

/// `a` ─ `b` ─ `c` (working copy), plus `d` branching off `a`. Each change
/// carries a bookmark of the same name so revsets can address it.
fn forked(repo: Repo) -> Repo {
    repo.file("f", "a\n")
        .describe("a")
        .bookmark("a")
        .new_change("b")
        .file("f", "b\n")
        .bookmark("b")
        .new_change("c")
        .file("f", "c\n")
        .bookmark("c")
        .new_change_on("a", "d")
        .file("g", "d\n")
        .bookmark("d")
        .jj(&["edit", "c"])
}

fn rebase(repo: &Repo, kind: RebaseKind, what: &str, emptied: EmptiedCommits) {
    let out = repo
        .cli()
        .rebase(&Rebase {
            kind,
            rev: rev(what),
            onto: rev("d"),
            emptied,
        })
        .unwrap();
    assert!(out.ok(), "{}", out.output);
}

fn parent(repo: &Repo, of: &str) -> String {
    repo.show(&format!("{of}-"), "description.first_line()")
}

#[test]
fn revision_moves_alone_and_its_child_closes_the_gap() {
    let repo = Repo::init();
    let repo = forked(repo);

    rebase(&repo, RebaseKind::Revision, "b", EmptiedCommits::Keep);

    assert_eq!(parent(&repo, "b"), "d");
    assert_eq!(parent(&repo, "c"), "a");
}

#[test]
fn with_descendants_moves_the_subtree() {
    let repo = Repo::init();
    let repo = forked(repo);

    rebase(
        &repo,
        RebaseKind::WithDescendants,
        "b",
        EmptiedCommits::Keep,
    );

    assert_eq!(parent(&repo, "b"), "d");
    assert_eq!(parent(&repo, "c"), "b");
}

#[test]
fn branch_moves_everything_not_already_on_the_destination() {
    let repo = Repo::init();
    let repo = forked(repo);

    rebase(&repo, RebaseKind::Branch, "c", EmptiedCommits::Keep);

    assert_eq!(parent(&repo, "b"), "d");
    assert_eq!(parent(&repo, "c"), "b");
}

#[test]
fn skip_emptied_abandons_a_change_the_destination_already_has() {
    let repo = Repo::init();
    // `d` carries the same edit as `b`, so `b` becomes empty on top of it.
    let repo = forked(repo)
        .jj(&["edit", "d"])
        .file("f", "b\n")
        .jj(&["edit", "c"]);

    rebase(&repo, RebaseKind::Revision, "b", EmptiedCommits::Skip);

    // The abandoned change's bookmark falls back onto its parent.
    assert_eq!(repo.show("b", "commit_id"), repo.show("d", "commit_id"));
}
