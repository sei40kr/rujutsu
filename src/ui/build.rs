//! Builders that turn jj snapshots into styled section trees. All buffer
//! kinds funnel through the same `Section` machinery in `section.rs`.
//! Colors come exclusively from `Theme` so the scheme is configurable.

use std::rc::Rc;

use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::jj::client::ProcessEntry;
use crate::jj::types::{DiffArea, FileDiff, LogRow, OpLogRow, RevEntry, StatusSnapshot};
use crate::theme::Theme;
use crate::ui::section::{Group, Section, SectionValue};

fn heading_style() -> Style {
    Style::new().add_modifier(Modifier::BOLD)
}

fn group_heading(t: &Theme, text: String) -> Line<'static> {
    Line::from(Span::styled(text, heading_style().fg(t.section_heading)))
}

pub fn build_status(t: &Theme, s: &StatusSnapshot) -> Section {
    let mut root = Section::root();
    root.body = header_lines(t, s).into_iter().map(Rc::new).collect();

    if !s.conflicts.is_empty() {
        let mut g = Section::new(
            0,
            "group:conflicts",
            SectionValue::Group(Group::Conflicts),
            group_heading(t, format!("Conflicts ({})", s.conflicts.len())),
        );
        for path in &s.conflicts {
            g.children.push(Section::new(
                g.id,
                &format!("conflict:{path}"),
                SectionValue::Text,
                Line::from(Span::styled(
                    format!("conflict   {path}"),
                    Style::new().fg(t.conflict),
                )),
            ));
        }
        root.children.push(g);
    }

    if !s.diff.is_empty() {
        let mut g = Section::new(
            0,
            "group:changes",
            SectionValue::Group(Group::Changes),
            group_heading(t, format!("Working copy changes ({})", s.diff.len())),
        );
        for fd in s.diff.iter() {
            g.children
                .push(file_section(t, g.id, DiffArea::WorkingCopy, fd));
        }
        root.children.push(g);
    }

    if !s.log.is_empty() {
        let mut g = Section::new(
            0,
            "group:log",
            SectionValue::Group(Group::Log),
            group_heading(t, "Log".to_string()),
        );
        push_log_rows(t, &mut g, &s.log);
        root.children.push(g);
    }

    if s.diff.is_empty() && s.conflicts.is_empty() && root.children.len() <= 1 {
        root.push_body(Line::default());
        root.push_body(Line::from(Span::styled(
            "The working copy has no changes.",
            Style::new().dim(),
        )));
    }
    root
}

fn header_lines(t: &Theme, s: &StatusSnapshot) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    if let Some(wc) = &s.working_copy {
        out.push(rev_header_line(t, "Working copy :", wc));
    }
    for p in &s.parents {
        out.push(rev_header_line(t, "Parent       :", p));
    }
    if !s.conflicts.is_empty() {
        out.push(Line::from(vec![
            Span::styled("State        : ".to_string(), Style::new().dim()),
            Span::styled(
                "unresolved conflicts".to_string(),
                Style::new().fg(t.conflict).bold(),
            ),
        ]));
    }
    out
}

/// `<label> <change-id> <commit-id> <bookmarks> <markers> <subject>` — the
/// status header's revision lines, mirroring `jj status` output.
fn rev_header_line(t: &Theme, label: &str, rev: &RevEntry) -> Line<'static> {
    let mut spans = vec![
        Span::styled(format!("{label} "), Style::new().dim()),
        Span::styled(rev.change_id.clone(), Style::new().fg(t.change_id).bold()),
        Span::raw(" "),
        Span::styled(rev.commit_id.clone(), Style::new().fg(t.commit_id)),
        Span::raw(" "),
    ];
    spans.extend(decoration_spans(t, rev));
    spans.push(subject_span(t, rev));
    Line::from(spans)
}

/// Bookmark/tag/state decorations shared by header and log rows.
fn decoration_spans(t: &Theme, rev: &RevEntry) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    for b in rev.bookmarks.split_whitespace() {
        out.push(Span::styled(
            b.to_string(),
            Style::new().fg(t.bookmark).bold(),
        ));
        out.push(Span::raw(" "));
    }
    for tag in rev.tags.split_whitespace() {
        out.push(Span::styled(tag.to_string(), Style::new().fg(t.tag).bold()));
        out.push(Span::raw(" "));
    }
    if rev.has_conflict {
        out.push(Span::styled(
            "(conflict)".to_string(),
            Style::new().fg(t.conflict),
        ));
        out.push(Span::raw(" "));
    }
    if rev.is_divergent {
        out.push(Span::styled(
            "(divergent)".to_string(),
            Style::new().fg(t.error),
        ));
        out.push(Span::raw(" "));
    }
    if rev.is_hidden {
        out.push(Span::styled("(hidden)".to_string(), Style::new().dim()));
        out.push(Span::raw(" "));
    }
    if rev.is_empty && !rev.is_root {
        out.push(Span::styled(
            "(empty)".to_string(),
            Style::new().fg(t.empty_marker),
        ));
        out.push(Span::raw(" "));
    }
    out
}

fn subject_span(t: &Theme, rev: &RevEntry) -> Span<'static> {
    if rev.is_root {
        Span::styled("root()".to_string(), Style::new().fg(t.immutable))
    } else if rev.subject.is_empty() {
        Span::styled(
            "(no description set)".to_string(),
            Style::new().fg(t.no_description),
        )
    } else {
        Span::raw(rev.subject.clone())
    }
}

/// Append log rows to `parent` as revision sections. Pure graph lines
/// ("│", "├─╯", "~") become body lines of the preceding revision, so n/p
/// stop only on revisions and the graph stays visually attached.
pub fn push_log_rows(t: &Theme, parent: &mut Section, rows: &[LogRow]) {
    // Column widths for the author/date margin, in display columns.
    let (author_col, date_col) = rows
        .iter()
        .filter_map(|r| match r {
            LogRow::Rev(e) => Some((e.author.width(), log_date(e).width())),
            LogRow::Graph(_) => None,
        })
        .fold((0, 0), |(a, d), (ea, ed)| (a.max(ea), d.max(ed)));

    let parent_id = parent.id;
    for row in rows {
        match row {
            LogRow::Rev(e) => {
                let mut sec = Section::new(
                    parent_id,
                    &format!("rev:{}", e.change_id),
                    SectionValue::Revision {
                        change_id: e.change_id.clone(),
                    },
                    rev_log_line(t, e),
                );
                sec.margin = log_margin(t, &e.author, author_col, log_date(e), date_col);
                parent.children.push(sec);
            }
            LogRow::Graph(g) => {
                let line = Line::from(Span::styled(g.clone(), Style::new().fg(t.graph)));
                match parent.children.last_mut() {
                    Some(last) => last.push_body(line),
                    None => parent.push_body(line),
                }
            }
        }
    }
}

/// One log row: graph prefix, change id, decorations, subject.
fn rev_log_line(t: &Theme, e: &RevEntry) -> Line<'static> {
    let graph_style = if e.is_working_copy {
        Style::new().fg(t.working_copy).bold()
    } else if e.is_immutable {
        Style::new().fg(t.immutable)
    } else {
        Style::new().fg(t.graph)
    };
    let id_style = if e.is_immutable {
        Style::new().fg(t.immutable).bold()
    } else {
        Style::new().fg(t.change_id).bold()
    };
    let mut spans = vec![
        Span::styled(e.graph.clone(), graph_style),
        Span::styled(e.change_id.clone(), id_style),
        Span::raw(" "),
    ];
    spans.extend(decoration_spans(t, e));
    let mut subject = subject_span(t, e);
    if e.is_working_copy {
        subject.style = subject.style.patch(heading_style());
    }
    spans.push(subject);
    Line::from(spans)
}

/// The date to show in the margin. The root commit's committer timestamp is the
/// Unix epoch, which jj renders as a meaningless "56 years ago" — suppress it.
fn log_date(e: &RevEntry) -> &str {
    if e.is_root {
        ""
    } else {
        &e.date
    }
}

/// The log's right-margin block: `author` (left-aligned in `author_col`, in the
/// `log_author` role), two spaces, then `date` (right-aligned in `date_col`, in
/// the `log_date` role). `None` when both are empty. Padding is by display
/// width so the columns stay aligned with wide glyphs.
fn log_margin(
    t: &Theme,
    author: &str,
    author_col: usize,
    date: &str,
    date_col: usize,
) -> Option<Line<'static>> {
    if author.is_empty() && date.is_empty() {
        return None;
    }
    Some(Line::from(vec![
        Span::styled(pad_end(author, author_col), Style::new().fg(t.log_author)),
        Span::raw("  "),
        Span::styled(pad_start(date, date_col), Style::new().fg(t.log_date)),
    ]))
}

/// Right-pad `s` with spaces to `cols` display columns (no-op if already wider).
fn pad_end(s: &str, cols: usize) -> String {
    let w = s.width();
    if w >= cols {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(cols - w))
    }
}

/// Left-pad `s` with spaces to `cols` display columns (no-op if already wider).
fn pad_start(s: &str, cols: usize) -> String {
    let w = s.width();
    if w >= cols {
        s.to_string()
    } else {
        format!("{}{s}", " ".repeat(cols - w))
    }
}

/// A file section with its hunks as children — shared by status and
/// revision buffers.
pub fn file_section(t: &Theme, parent_id: u64, area: DiffArea, fd: &FileDiff) -> Section {
    let name = match &fd.old_path {
        Some(old) => format!("{old} -> {}", fd.path),
        None => fd.path.clone(),
    };
    let mut sec = Section::new(
        parent_id,
        &format!("file:{}", fd.path),
        SectionValue::File {
            area,
            path: fd.path.clone(),
        },
        Line::from(vec![
            Span::styled(
                format!("{}   ", fd.status_word()),
                Style::new().fg(t.file_status),
            ),
            Span::styled(name, heading_style()),
        ]),
    );
    if fd.is_binary {
        sec.push_body(Line::from(Span::styled(
            "(binary file)".to_string(),
            Style::new().dim(),
        )));
    }
    for (hunk_idx, hunk) in fd.hunks.iter().enumerate() {
        let mut h = Section::new(
            sec.id,
            &format!("hunk:{}", hunk.old_start),
            SectionValue::Hunk {
                area,
                path: fd.path.clone(),
                hunk_idx,
            },
            Line::from(Span::styled(
                hunk.header.clone(),
                Style::new().fg(t.hunk_header),
            )),
        );
        for l in &hunk.lines {
            h.push_body(diff_line(t, l));
        }
        sec.children.push(h);
    }
    sec
}

fn diff_line(t: &Theme, l: &str) -> Line<'static> {
    let style = match l.as_bytes().first() {
        Some(b'+') => Style::new().fg(t.diff_add),
        Some(b'-') => Style::new().fg(t.diff_remove),
        Some(b'\\') => Style::new().dim(),
        _ => Style::new(),
    };
    Line::from(Span::styled(l.to_string(), style))
}

/// Revision buffer (RET on a revision): header text followed by the diff as
/// read-only file sections.
pub fn build_revision(t: &Theme, header: &str, files: &[FileDiff]) -> Section {
    let mut root = Section::root();
    root.body = header
        .lines()
        .map(|l| Rc::new(Line::from(l.to_string())))
        .collect();
    for fd in files.iter() {
        root.children
            .push(file_section(t, 0, DiffArea::Committed, fd));
    }
    root
}

/// Log buffer: revisions as top-level sections under a full-width header bar.
pub fn build_log(t: &Theme, title: &str, rows: &[LogRow]) -> Section {
    let mut root = Section::root();
    // Revisions are top-level sections; render them as one tight list rather
    // than blank-line-separated like status groups.
    root.compact = true;
    root.body_fill = Some(t.header_bg);
    root.push_body(Line::from(Span::styled(
        title.to_string(),
        heading_style().fg(t.header_fg).bg(t.header_bg),
    )));
    if rows.is_empty() {
        root.push_body(Line::from(Span::styled(
            "No revisions.".to_string(),
            Style::new().dim(),
        )));
        return root;
    }
    push_log_rows(t, &mut root, rows);
    root
}

/// Operation log buffer: operations as top-level sections.
pub fn build_op_log(t: &Theme, rows: &[OpLogRow]) -> Section {
    let mut root = Section::root();
    root.compact = true;
    root.body_fill = Some(t.header_bg);
    root.push_body(Line::from(Span::styled(
        "Operation log".to_string(),
        heading_style().fg(t.header_fg).bg(t.header_bg),
    )));
    for row in rows {
        match row {
            OpLogRow::Op(op) => {
                let graph_style = if op.is_current {
                    Style::new().fg(t.working_copy).bold()
                } else {
                    Style::new().fg(t.graph)
                };
                let mut sec = Section::new(
                    0,
                    &format!("op:{}", op.id),
                    SectionValue::Operation {
                        op_id: op.id.clone(),
                    },
                    Line::from(vec![
                        Span::styled(op.graph.clone(), graph_style),
                        Span::styled(op.id.clone(), Style::new().fg(t.op_id)),
                        Span::raw(" "),
                        Span::raw(op.description.clone()),
                    ]),
                );
                sec.margin = Some(Line::from(Span::styled(
                    op.time.clone(),
                    Style::new().fg(t.log_date),
                )));
                root.children.push(sec);
            }
            OpLogRow::Graph(g) => {
                let line = Line::from(Span::styled(g.clone(), Style::new().fg(t.graph)));
                match root.children.last_mut() {
                    Some(last) => last.push_body(line),
                    None => root.push_body(line),
                }
            }
        }
    }
    root
}

/// Plain-text buffer (`jj op show` output).
pub fn build_text(title: &str, t: &Theme, text: &str) -> Section {
    let mut root = Section::root();
    root.compact = true;
    // `body_fill` paints the whole root body as a full-width bar, so only the
    // title lives there. The output goes in child sections, which render on
    // the normal background — otherwise every line looks highlighted.
    root.body_fill = Some(t.header_bg);
    root.push_body(Line::from(Span::styled(
        title.to_string(),
        heading_style().fg(t.header_fg).bg(t.header_bg),
    )));
    for (i, l) in text.lines().enumerate() {
        root.children.push(Section::new(
            root.id,
            &format!("line:{i}"),
            SectionValue::Text,
            Line::from(l.to_string()),
        ));
    }
    root
}

/// The `$` buffer: every jj command run by the app, newest last.
/// `first_index` numbers the section keys when old entries have been
/// trimmed, so surviving entries keep their identity (fold state, cursor).
pub fn build_process_log(t: &Theme, entries: &[ProcessEntry], first_index: usize) -> Section {
    let mut root = Section::root();
    if entries.is_empty() {
        root.push_body(Line::from(Span::styled(
            "No jj commands run yet.".to_string(),
            Style::new().dim(),
        )));
        return root;
    }
    for (i, e) in entries.iter().enumerate() {
        let status_style = if e.status == 0 {
            Style::new().fg(t.success)
        } else {
            Style::new().fg(t.error)
        };
        let mut sec = Section::new(
            0,
            &format!("proc:{}", first_index + i),
            SectionValue::Text,
            Line::from(vec![
                Span::styled(format!("[{}] ", e.status), status_style),
                Span::styled(e.cmd.clone(), heading_style()),
            ]),
        );
        for l in e.output.lines() {
            sec.push_body(Line::from(Span::styled(l.to_string(), Style::new().dim())));
        }
        root.children.push(sec);
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jj::types::OpEntry;

    fn rev(change_id: &str, subject: &str) -> RevEntry {
        RevEntry {
            graph: "○  ".into(),
            change_id: change_id.into(),
            commit_id: "abcd1234".into(),
            subject: subject.into(),
            author: "Ada".into(),
            date: "2 days ago".into(),
            ..RevEntry::default()
        }
    }

    #[test]
    fn log_rows_become_revision_sections_with_margins() {
        let t = Theme::default();
        let rows = vec![
            LogRow::Rev(rev("abc", "first")),
            LogRow::Graph("│".into()),
            LogRow::Rev(rev("def", "second")),
        ];
        let root = build_log(&t, "Log", &rows);
        assert_eq!(root.children.len(), 2);
        assert_eq!(
            root.children[0].value,
            SectionValue::Revision {
                change_id: "abc".into()
            }
        );
        // The graph line attaches to the preceding revision's body.
        assert_eq!(root.children[0].body[0].to_string(), "│");
        let margin = root.children[0].margin.as_ref().unwrap();
        assert_eq!(margin.to_string(), "Ada  2 days ago");
    }

    #[test]
    fn working_copy_row_is_bold_and_marked() {
        let t = Theme::default();
        let mut e = rev("abc", "wip");
        e.graph = "@  ".into();
        e.is_working_copy = true;
        let root = build_log(&t, "Log", &[LogRow::Rev(e)]);
        let heading = &root.children[0].heading;
        assert_eq!(heading.spans[0].style.fg, Some(t.working_copy));
    }

    #[test]
    fn empty_and_undescribed_markers() {
        let t = Theme::default();
        let mut e = rev("abc", "");
        e.is_empty = true;
        let root = build_log(&t, "Log", &[LogRow::Rev(e)]);
        let text = root.children[0].heading.to_string();
        assert!(text.contains("(empty)"));
        assert!(text.contains("(no description set)"));
    }

    #[test]
    fn root_rev_shows_root_marker_without_noise() {
        let t = Theme::default();
        let mut e = rev("zzz", "");
        e.is_root = true;
        e.is_empty = true;
        e.is_immutable = true;
        e.author = String::new();
        e.date = "56 years ago".into();
        let root = build_log(&t, "Log", &[LogRow::Rev(e)]);
        let text = root.children[0].heading.to_string();
        assert!(text.contains("root()"));
        assert!(!text.contains("(empty)"));
        assert!(!text.contains("(no description set)"));
        // The root's bogus committer timestamp must not leak into the margin.
        assert!(root.children[0].margin.is_none());
    }

    #[test]
    fn status_header_names_working_copy_and_parents() {
        let t = Theme::default();
        let mut wc = rev("sym", "wip");
        wc.is_working_copy = true;
        let snapshot = StatusSnapshot {
            working_copy: Some(wc),
            parents: vec![rev("twm", "first")],
            ..StatusSnapshot::default()
        };
        let root = build_status(&t, &snapshot);
        assert!(root.body[0].to_string().starts_with("Working copy :"));
        assert!(root.body[1].to_string().starts_with("Parent       :"));
    }

    #[test]
    fn status_groups_conflicts_changes_and_log() {
        let t = Theme::default();
        let snapshot = StatusSnapshot {
            working_copy: Some(rev("sym", "wip")),
            conflicts: vec!["src/main.rs".into()],
            diff: std::sync::Arc::new(vec![FileDiff {
                path: "a.txt".into(),
                ..FileDiff::default()
            }]),
            log: vec![LogRow::Rev(rev("abc", "first"))],
            ..StatusSnapshot::default()
        };
        let root = build_status(&t, &snapshot);
        let headings: Vec<String> = root
            .children
            .iter()
            .map(|c| c.heading.to_string())
            .collect();
        assert_eq!(
            headings,
            vec!["Conflicts (1)", "Working copy changes (1)", "Log"]
        );
    }

    #[test]
    fn op_log_rows_become_operation_sections() {
        let t = Theme::default();
        let rows = vec![
            OpLogRow::Op(OpEntry {
                graph: "@  ".into(),
                id: "abc123".into(),
                description: "snapshot working copy".into(),
                time: "5 minutes ago".into(),
                is_current: true,
            }),
            OpLogRow::Graph("│".into()),
        ];
        let root = build_op_log(&t, &rows);
        assert_eq!(
            root.children[0].value,
            SectionValue::Operation {
                op_id: "abc123".into()
            }
        );
        assert_eq!(root.children[0].body[0].to_string(), "│");
    }

    #[test]
    fn op_show_fills_only_the_title_not_the_body() {
        let t = Theme::default();
        let root = build_text("operation abc123", &t, "line one\nline two");
        // Content is children, so the header bar covers only the title.
        assert_eq!(root.body.len(), 1);
        assert_eq!(root.children.len(), 2);

        let flat = crate::ui::section::flatten(&root);
        let filled: Vec<String> = flat
            .iter()
            .filter(|f| f.fill_bg.is_some())
            .map(|f| f.line.to_string())
            .collect();
        assert_eq!(filled, vec!["operation abc123"]);
        // The output lines render, unfilled.
        let texts: Vec<String> = flat.iter().map(|f| f.line.to_string()).collect();
        assert_eq!(texts, vec!["operation abc123", "line one", "line two"]);
    }
}
