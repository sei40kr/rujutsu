use rujutsu::jj::op::{BookmarkName, BookmarkOp, RemoteBookmark};
use rujutsu::jj::Jj;

use crate::common::{rev, Repo};

fn name(s: &str) -> BookmarkName {
    BookmarkName::parse(s).unwrap()
}

fn bookmark(repo: &Repo, op: BookmarkOp) {
    let out = repo.cli().bookmark(&op).unwrap();
    assert!(out.ok(), "{}", out.output);
}

#[test]
fn create_points_at_the_revision() {
    let repo = Repo::init().describe("a").new_change("b");

    bookmark(
        &repo,
        BookmarkOp::Create {
            name: name("feature"),
            at: rev("@-"),
        },
    );

    assert_eq!(repo.bookmarks(), ["feature"]);
    assert_eq!(repo.show("feature", "description.first_line()"), "a");
}

#[test]
fn set_moves_an_existing_bookmark() {
    let repo = Repo::init().describe("a").bookmark("main").new_change("b");

    bookmark(
        &repo,
        BookmarkOp::Set {
            name: name("main"),
            at: rev("@"),
        },
    );

    assert_eq!(repo.show("main", "description.first_line()"), "b");
}

#[test]
fn rename_delete_and_forget_edit_the_list() {
    let repo = Repo::init().describe("a").bookmark("one").bookmark("two");

    bookmark(
        &repo,
        BookmarkOp::Rename {
            from: name("one"),
            to: name("uno"),
        },
    );
    assert_eq!(repo.bookmarks(), ["two", "uno"]);

    bookmark(&repo, BookmarkOp::Delete(name("two")));
    assert_eq!(repo.bookmarks(), ["uno"]);

    bookmark(&repo, BookmarkOp::Forget(name("uno")));
    assert!(repo.bookmarks().is_empty());
}

#[test]
fn untrack_and_track_toggle_a_remote_bookmark() {
    let remote = Repo::init();
    let repo = Repo::init()
        .remote(&remote)
        .describe("a")
        .bookmark("main")
        .jj(&["git", "push", "--bookmark", "main"]);
    assert_eq!(repo.bookmarks(), ["main", "main@origin*"]);
    let b = RemoteBookmark::parse("main@origin").unwrap();

    bookmark(&repo, BookmarkOp::Untrack(b.clone()));
    assert_eq!(repo.bookmarks(), ["main", "main@origin"]);

    bookmark(&repo, BookmarkOp::Track(b));
    assert_eq!(repo.bookmarks(), ["main", "main@origin*"]);
}
