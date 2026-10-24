use rujutsu::jj::op::Fetch;
use rujutsu::jj::Jj;

use crate::common::Repo;

fn fetch(op: Fetch) {
    let remote = Repo::init();
    // One clone publishes `main`; a second one fetches it.
    let _publisher = Repo::init()
        .remote(&remote)
        .describe("a")
        .bookmark("main")
        .jj(&["git", "push", "--bookmark", "main", "--allow-new"]);
    let repo = Repo::init().remote(&remote);
    assert!(repo.bookmarks().is_empty());

    let out = repo.cli().fetch(&op).unwrap();

    assert!(out.ok(), "{}", out.output);
    // Fetched bookmarks arrive untracked.
    assert_eq!(repo.bookmarks(), ["main@origin"]);
}

#[test]
fn default_remote() {
    fetch(Fetch::DefaultRemote);
}

#[test]
fn all_remotes() {
    fetch(Fetch::AllRemotes);
}
