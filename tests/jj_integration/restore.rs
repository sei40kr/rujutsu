use rujutsu::jj::op::{RepoPath, Restore};
use rujutsu::jj::Jj;

use crate::common::Repo;

fn dirty(repo: Repo) -> Repo {
    repo.file("a.txt", "one\n")
        .file("b.txt", "one\n")
        .describe("base")
        .new_change("wip")
        .file("a.txt", "two\n")
        .file("b.txt", "two\n")
}

#[test]
fn path_discards_one_file() {
    let repo = Repo::init();
    let repo = dirty(repo);

    let out = repo
        .cli()
        .restore(&Restore::Path(RepoPath::parse("a.txt").unwrap()))
        .unwrap();

    assert!(out.ok(), "{}", out.output);
    assert_eq!(repo.read("a.txt"), "one\n");
    assert_eq!(repo.read("b.txt"), "two\n");
}

#[test]
fn working_copy_discards_everything() {
    let repo = Repo::init();
    let repo = dirty(repo);

    let out = repo.cli().restore(&Restore::WorkingCopy).unwrap();

    assert!(out.ok(), "{}", out.output);
    assert_eq!(repo.read("a.txt"), "one\n");
    assert_eq!(repo.read("b.txt"), "one\n");
    assert!(repo.changed_paths("@").is_empty());
}
