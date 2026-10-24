use rujutsu::jj::Jj;

use crate::common::{rev, Repo};

#[test]
fn revision_reads_header_and_parsed_diff() {
    let repo = Repo::init()
        .file("a.txt", "hello\n")
        .describe("first change")
        .new_change("second change");

    let view = repo.cli().revision(&rev("@-")).unwrap();

    assert!(view.header.contains("first change"));
    assert_eq!(view.files.len(), 1);
    assert_eq!(view.files[0].path.as_str(), "a.txt");
    assert!(view.files[0].is_new);
    assert!(view.files[0].hunks[0].lines.iter().any(|l| l == "+hello"));
}
