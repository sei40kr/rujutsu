use rujutsu::jj::op::{BookmarkName, Deletions, Push, PushOptions, PushScope, RunMode};
use rujutsu::jj::Jj;

use crate::common::{rev, Repo};

/// A repo with `main` at a described change and `origin` pointing at
/// `remote`'s git store.
fn with_remote(remote: &Repo) -> Repo {
    Repo::init()
        .remote(remote)
        .file("a.txt", "hello\n")
        .describe("a")
        .bookmark("main")
}

/// What the remote has, seen from its own jj repo.
fn remote_bookmarks(remote: &Repo) -> Vec<String> {
    // Pushes land in the git store; import them into the jj view.
    std::process::Command::new("jj")
        .args(["git", "import"])
        .current_dir(remote.path())
        .output()
        .unwrap();
    remote.bookmarks()
}

fn push(repo: &Repo, scope: PushScope, options: PushOptions) -> rujutsu::jj::Outcome {
    repo.cli().push(&Push { scope, options }).unwrap()
}

#[test]
fn new_bookmark_lands_on_the_remote() {
    let remote = Repo::init();
    let repo = with_remote(&remote);

    let out = push(
        &repo,
        PushScope::Bookmark(BookmarkName::parse("main").unwrap()),
        PushOptions::default(),
    );
    assert!(out.ok(), "{}", out.output);
    assert_eq!(remote_bookmarks(&remote), ["main"]);
}

#[test]
fn tracked_pushes_what_the_remote_already_follows() {
    let remote = Repo::init();
    let repo = with_remote(&remote)
        .jj(&["git", "push", "--bookmark", "main"])
        // The pushed change became immutable, so this edits a fresh @.
        .file("a.txt", "hello\nmore\n")
        .describe("more")
        .jj(&["bookmark", "set", "main", "-r", "@"]);

    let out = push(&repo, PushScope::Tracked, PushOptions::default());

    assert!(out.ok(), "{}", out.output);
    assert_eq!(
        repo.show("main@origin", "commit_id"),
        repo.show("main", "commit_id")
    );
}

#[test]
fn change_pushes_under_a_generated_bookmark() {
    let remote = Repo::init();
    let repo = with_remote(&remote);

    let out = push(&repo, PushScope::Change(rev("@")), PushOptions::default());

    assert!(out.ok(), "{}", out.output);
    assert!(remote_bookmarks(&remote)
        .iter()
        .any(|b| b.starts_with("push-")));
}

#[test]
fn dry_run_touches_nothing() {
    let remote = Repo::init();
    let repo = with_remote(&remote);

    let out = push(
        &repo,
        PushScope::Bookmark(BookmarkName::parse("main").unwrap()),
        PushOptions {
            mode: RunMode::DryRun,
            ..PushOptions::default()
        },
    );

    assert!(out.ok(), "{}", out.output);
    assert!(remote_bookmarks(&remote).is_empty());
}

#[test]
fn propagating_deletions_removes_the_remote_bookmark() {
    let remote = Repo::init();
    let repo = with_remote(&remote)
        .jj(&["git", "push", "--bookmark", "main"])
        .jj(&["bookmark", "delete", "main"]);

    let out = push(
        &repo,
        PushScope::Tracked,
        PushOptions {
            deletions: Deletions::Propagate,
            ..PushOptions::default()
        },
    );

    assert!(out.ok(), "{}", out.output);
    assert!(remote_bookmarks(&remote).is_empty());
}
