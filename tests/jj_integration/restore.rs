use rujutsu::jj::op::{Fileset, RepoPath, Restore, RestoreDescendants, RestoreSource, RestoreSpec};
use rujutsu::jj::Jj;

use crate::common::{rev, Repo};

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

fn spec(source: RestoreSource, fileset: Option<&str>) -> Restore {
    Restore::Spec(RestoreSpec {
        source,
        fileset: fileset.map(|f| Fileset::parse(f).unwrap()),
        descendants: RestoreDescendants::default(),
    })
}

#[test]
fn spec_restores_a_fileset_from_a_revision() {
    let repo = Repo::init();
    let repo = dirty(repo);

    let source = RestoreSource::Between {
        from: Some(rev("@-")),
        into: None,
    };
    let out = repo.cli().restore(&spec(source, Some("b.txt"))).unwrap();

    assert!(out.ok(), "{}", out.output);
    assert_eq!(repo.read("a.txt"), "two\n");
    assert_eq!(repo.read("b.txt"), "one\n");
}

#[test]
fn changes_in_undoes_a_revision() {
    let repo = Repo::init();
    // Undo the wip change from a child, so restoring rewrites a non-@ revision.
    let repo = dirty(repo).new_change("child");

    let out = repo
        .cli()
        .restore(&spec(RestoreSource::ChangesIn(rev("@-")), None))
        .unwrap();

    assert!(out.ok(), "{}", out.output);
    assert!(repo.changed_paths("@-").is_empty());
    assert_eq!(repo.read("a.txt"), "one\n");
}
