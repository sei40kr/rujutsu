//! Pure parsers for jj output. No I/O here — everything is unit-testable
//! against fixture strings.
//!
//! Log and op-log rows are produced by templates that start each record with
//! a NUL byte (see `client`), so a line containing NUL is a data row — the
//! text before the first NUL is the graph prefix — and a line without NUL is
//! a pure graph line ("│", "├─╯", "~").

use super::types::{FileDiff, Hunk, LogRow, OpEntry, OpLogRow, RevEntry};

/// Parse the output of `jj log`/`jj evolog` run with `LOG_TEMPLATE`.
pub fn parse_log_rows(text: &str) -> Vec<LogRow> {
    text.lines()
        .map(|line| match parse_rev_line(line) {
            Some(rev) => LogRow::Rev(rev),
            None => LogRow::Graph(line.to_string()),
        })
        .collect()
}

/// Like `parse_log_rows` but keeps only the revisions (for `--no-graph`
/// reads such as the working-copy header).
pub fn parse_rev_entries(text: &str) -> Vec<RevEntry> {
    text.lines().filter_map(parse_rev_line).collect()
}

fn parse_rev_line(line: &str) -> Option<RevEntry> {
    let (graph, rest) = line.split_once('\0')?;
    let mut f = rest.split('\0');
    let change_id = f.next()?.to_string();
    if change_id.is_empty() {
        return None;
    }
    let commit_id = f.next().unwrap_or("").to_string();
    let bookmarks = f.next().unwrap_or("").to_string();
    let tags = f.next().unwrap_or("").to_string();
    let subject = f.next().unwrap_or("").to_string();
    let author = f.next().unwrap_or("").to_string();
    let date = f.next().unwrap_or("").to_string();
    let flags = f.next().unwrap_or("");
    Some(RevEntry {
        graph: graph.to_string(),
        change_id,
        commit_id,
        bookmarks,
        tags,
        subject,
        author,
        date,
        is_working_copy: flags.contains('@'),
        has_conflict: flags.contains('C'),
        is_empty: flags.contains('E'),
        is_immutable: flags.contains('I'),
        is_root: flags.contains('R'),
        is_divergent: flags.contains('D'),
        is_hidden: flags.contains('H'),
    })
}

/// Parse the output of `jj op log` run with `OP_TEMPLATE`.
pub fn parse_op_rows(text: &str) -> Vec<OpLogRow> {
    text.lines()
        .map(|line| match parse_op_line(line) {
            Some(op) => OpLogRow::Op(op),
            None => OpLogRow::Graph(line.to_string()),
        })
        .collect()
}

fn parse_op_line(line: &str) -> Option<OpEntry> {
    let (graph, rest) = line.split_once('\0')?;
    let mut f = rest.split('\0');
    let id = f.next()?.to_string();
    if id.is_empty() {
        return None;
    }
    Some(OpEntry {
        graph: graph.to_string(),
        id,
        description: f.next().unwrap_or("").to_string(),
        time: f.next().unwrap_or("").to_string(),
        is_current: f.next().unwrap_or("").contains('@'),
    })
}

/// Parse `jj bookmark list -T '...'` output: `name NUL remote` per line.
pub fn parse_bookmarks(text: &str) -> Vec<super::types::BookmarkInfo> {
    text.lines()
        .filter_map(|l| {
            let (name, remote) = l.split_once('\0')?;
            if name.is_empty() {
                return None;
            }
            Some(super::types::BookmarkInfo {
                name: name.to_string(),
                remote: (!remote.is_empty()).then(|| remote.to_string()),
            })
        })
        .collect()
}

/// Parse `jj resolve --list` output: one `path    description` per line.
/// (jj exits non-zero when there are no conflicts; callers pass "" then.)
pub fn parse_conflict_list(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| {
            let path = l.split_whitespace().next()?;
            (!path.is_empty()).then(|| path.to_string())
        })
        .collect()
}

/// Parse `jj diff --git` unified output into per-file diffs with hunks.
pub fn parse_diff(text: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut cur: Option<FileDiff> = None;
    let mut cur_hunk: Option<Hunk> = None;

    let flush_hunk = |cur: &mut Option<FileDiff>, hunk: &mut Option<Hunk>| {
        if let (Some(f), Some(h)) = (cur.as_mut(), hunk.take()) {
            f.hunks.push(h);
        }
    };

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            flush_hunk(&mut cur, &mut cur_hunk);
            if let Some(f) = cur.take() {
                files.push(f);
            }
            let mut fd = FileDiff::default();
            // Fallback path from the `diff --git a/X b/Y` line; the
            // `+++`/`---` header lines below take precedence.
            if let Some((_, b)) = rest.split_once(" b/") {
                fd.path = unquote(b);
            }
            cur = Some(fd);
            continue;
        }
        let Some(fd) = cur.as_mut() else { continue };

        if let Some(hunk) = cur_hunk
            .as_mut()
            .filter(|_| matches!(line.as_bytes().first(), Some(b' ' | b'+' | b'-' | b'\\')))
        {
            hunk.lines.push(line.to_string());
        } else if line.starts_with("@@") {
            flush_hunk(&mut cur, &mut cur_hunk);
            cur_hunk = parse_hunk_header(line);
        } else if line.starts_with("new file mode") {
            fd.is_new = true;
        } else if line.starts_with("deleted file mode") {
            fd.is_deleted = true;
        } else if let Some(p) = line.strip_prefix("rename from ") {
            fd.old_path = Some(unquote(p));
        } else if let Some(p) = line.strip_prefix("rename to ") {
            fd.path = unquote(p);
        } else if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
            fd.is_binary = true;
        } else if let Some(p) = line.strip_prefix("--- a/") {
            if fd.old_path.is_none() && !fd.is_new {
                fd.old_path = Some(unquote(p)).filter(|op| *op != fd.path);
            }
        } else if let Some(p) = line.strip_prefix("+++ b/") {
            fd.path = unquote(p);
        }
        // "index ...", "old mode", "similarity index", "--- /dev/null",
        // "+++ /dev/null" need no handling.
    }
    flush_hunk(&mut cur, &mut cur_hunk);
    if let Some(f) = cur.take() {
        files.push(f);
    }
    files
}

/// Parse `@@ -a,b +c,d @@ ctx` (counts default to 1 when omitted).
fn parse_hunk_header(line: &str) -> Option<Hunk> {
    let rest = line.strip_prefix("@@ -")?;
    let (old, rest) = rest.split_once(" +")?;
    let (new, _) = rest.split_once(" @@")?;
    let parse_pair = |s: &str| -> Option<(u32, u32)> {
        match s.split_once(',') {
            Some((a, b)) => Some((a.parse().ok()?, b.parse().ok()?)),
            None => Some((s.parse().ok()?, 1)),
        }
    };
    let (old_start, old_count) = parse_pair(old)?;
    let (new_start, new_count) = parse_pair(new)?;
    Some(Hunk {
        old_start,
        old_count,
        new_start,
        new_count,
        header: line.to_string(),
        lines: Vec::new(),
    })
}

/// Minimal unquoting of quoted paths (`"path with \"quotes\""`).
fn unquote(s: &str) -> String {
    if !(s.starts_with('"') && s.ends_with('"') && s.len() >= 2) {
        return s.to_string();
    }
    let mut out = String::new();
    let mut chars = s[1..s.len() - 1].chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_rows_split_graph_and_fields() {
        let raw = "@\u{0}sym\u{0}4c39\u{0}\u{0}\u{0}wip\u{0}Ada\u{0}1 hour ago\u{0}@\n\
                   │ ○  \u{0}abc\u{0}1111\u{0}main\u{0}v1\u{0}first\u{0}Ada\u{0}2 days ago\u{0}\n\
                   ├─╯\n\
                   ◆  \u{0}zzz\u{0}0000\u{0}\u{0}\u{0}\u{0}\u{0}56 years ago\u{0}EIR\n";
        let rows = parse_log_rows(raw);
        assert_eq!(rows.len(), 4);
        let LogRow::Rev(wc) = &rows[0] else {
            panic!("expected rev")
        };
        assert_eq!(wc.graph, "@");
        assert_eq!(wc.change_id, "sym");
        assert!(wc.is_working_copy);
        assert_eq!(wc.subject, "wip");
        let LogRow::Rev(main) = &rows[1] else {
            panic!("expected rev")
        };
        assert_eq!(main.graph, "│ ○  ");
        assert_eq!(main.bookmarks, "main");
        assert_eq!(main.tags, "v1");
        assert_eq!(rows[2], LogRow::Graph("├─╯".to_string()));
        let LogRow::Rev(root) = &rows[3] else {
            panic!("expected rev")
        };
        assert!(root.is_empty && root.is_immutable && root.is_root);
        assert!(!root.is_working_copy);
    }

    #[test]
    fn rev_entries_skip_graph_lines() {
        let raw = "~\n\u{0}abc\u{0}111\u{0}\u{0}\u{0}s\u{0}A\u{0}now\u{0}\n";
        let entries = parse_rev_entries(raw);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].change_id, "abc");
        assert_eq!(entries[0].graph, "");
    }

    #[test]
    fn op_rows_parse_fields_and_current_marker() {
        let raw = "@  \u{0}63130065\u{0}snapshot working copy\u{0}5 minutes ago\u{0}@\n\
                   ○  \u{0}7694ef02\u{0}new empty commit\u{0}6 minutes ago\u{0}\n";
        let rows = parse_op_rows(raw);
        let OpLogRow::Op(cur) = &rows[0] else {
            panic!("expected op")
        };
        assert!(cur.is_current);
        assert_eq!(cur.id, "63130065");
        assert_eq!(cur.description, "snapshot working copy");
        let OpLogRow::Op(prev) = &rows[1] else {
            panic!("expected op")
        };
        assert!(!prev.is_current);
    }

    #[test]
    fn bookmarks_local_and_remote() {
        let raw = "main\u{0}\nmain\u{0}origin\nfeature\u{0}\n";
        let bms = parse_bookmarks(raw);
        assert_eq!(bms.len(), 3);
        assert_eq!(bms[0].name, "main");
        assert_eq!(bms[0].remote, None);
        assert_eq!(bms[1].remote.as_deref(), Some("origin"));
    }

    #[test]
    fn conflict_list_takes_first_column() {
        let raw = "src/main.rs    2-sided conflict\nREADME.md    2-sided conflict\n";
        assert_eq!(parse_conflict_list(raw), vec!["src/main.rs", "README.md"]);
    }

    #[test]
    fn diff_two_files_with_hunks() {
        let raw = "\
diff --git a/src/a.rs b/src/a.rs
index 111..222 100644
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,3 +1,4 @@ fn main
 line1
+added
 line2
 line3
@@ -10,2 +11,2 @@
-old
+new
 ctx
diff --git a/b.txt b/b.txt
new file mode 100644
--- /dev/null
+++ b/b.txt
@@ -0,0 +1 @@
+hello
";
        let files = parse_diff(raw);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "src/a.rs");
        assert_eq!(files[0].hunks.len(), 2);
        assert_eq!(files[0].hunks[0].old_start, 1);
        assert_eq!(files[0].hunks[0].new_count, 4);
        assert_eq!(files[0].hunks[0].lines.len(), 4);
        assert_eq!(files[0].hunks[1].lines, vec!["-old", "+new", " ctx"]);
        assert!(files[1].is_new);
        assert_eq!(files[1].path, "b.txt");
        assert_eq!(files[1].hunks[0].lines, vec!["+hello"]);
    }

    #[test]
    fn diff_rename_and_binary() {
        let raw = "\
diff --git a/old.rs b/new.rs
similarity index 90%
rename from old.rs
rename to new.rs
diff --git a/img.png b/img.png
index 111..222 100644
Binary files a/img.png and b/img.png differ
";
        let files = parse_diff(raw);
        assert_eq!(files[0].old_path.as_deref(), Some("old.rs"));
        assert_eq!(files[0].path, "new.rs");
        assert!(files[1].is_binary);
    }

    #[test]
    fn hunk_header_without_counts() {
        let h = parse_hunk_header("@@ -5 +7 @@").unwrap();
        assert_eq!(
            (h.old_start, h.old_count, h.new_start, h.new_count),
            (5, 1, 7, 1)
        );
    }
}
