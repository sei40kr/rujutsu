//! Shelling out to the `jj` CLI. Reads run with `--ignore-working-copy` so
//! they never snapshot (which would write an operation and re-trigger the fs
//! watcher); the one exception is the working-copy read at the top of
//! `read_snapshot`, which deliberately snapshots so external file edits show
//! up.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use super::parse;
use super::types::{LogRow, OpLogRow, StatusSnapshot};

/// Template for one log/evolog row. Starts each record with NUL so the
/// parser can split the graph prefix from the fields, and NUL-separates the
/// fields (descriptions cannot contain NUL). The trailing flag field packs
/// booleans as single letters. Graph mode adds line breaks itself; the
/// `--no-graph` variant appends its own.
pub const LOG_TEMPLATE: &str = concat!(
    "concat(\"\\0\", change_id.shortest(8), \"\\0\", commit_id.shortest(8), \"\\0\", ",
    "bookmarks.join(\" \"), \"\\0\", tags.join(\" \"), \"\\0\", ",
    "if(description, description.first_line(), \"\"), \"\\0\", ",
    "author.name(), \"\\0\", committer.timestamp().ago(), \"\\0\", ",
    "if(current_working_copy, \"@\", \"\"), if(conflict, \"C\", \"\"), ",
    "if(empty, \"E\", \"\"), if(immutable, \"I\", \"\"), if(root, \"R\", \"\"), ",
    "if(divergent, \"D\", \"\"), if(hidden, \"H\", \"\"))",
);

/// `LOG_TEMPLATE` with an explicit newline, for `--no-graph` reads.
pub fn log_template_no_graph() -> String {
    format!("{LOG_TEMPLATE} ++ \"\\n\"")
}

/// `LOG_TEMPLATE` for `jj evolog`, whose rows are evolution entries that
/// expose the commit behind a `commit` keyword instead of directly.
pub const EVOLOG_TEMPLATE: &str = concat!(
    "concat(\"\\0\", commit.change_id().shortest(8), \"\\0\", ",
    "commit.commit_id().shortest(8), \"\\0\", ",
    "commit.bookmarks().join(\" \"), \"\\0\", commit.tags().join(\" \"), \"\\0\", ",
    "if(commit.description(), commit.description().first_line(), \"\"), \"\\0\", ",
    "commit.author().name(), \"\\0\", commit.committer().timestamp().ago(), \"\\0\", ",
    "if(commit.current_working_copy(), \"@\", \"\"), if(commit.conflict(), \"C\", \"\"), ",
    "if(commit.empty(), \"E\", \"\"), if(commit.immutable(), \"I\", \"\"), ",
    "if(commit.root(), \"R\", \"\"), ",
    "if(commit.divergent(), \"D\", \"\"), if(commit.hidden(), \"H\", \"\"))",
);

/// Template for one op-log row; same NUL-framing as `LOG_TEMPLATE`.
pub const OP_TEMPLATE: &str = concat!(
    "concat(\"\\0\", self.id().short(12), \"\\0\", self.description(), \"\\0\", ",
    "self.time().start().ago(), \"\\0\", if(current_operation, \"@\", \"\"))",
);

/// Multi-line header template for the revision buffer.
pub const SHOW_TEMPLATE: &str = concat!(
    "concat(",
    "\"Change ID: \", change_id, \"\\n\", ",
    "\"Commit ID: \", commit_id, \"\\n\", ",
    "if(bookmarks, concat(\"Bookmarks: \", bookmarks.join(\" \"), \"\\n\"), \"\"), ",
    "if(tags, concat(\"Tags:      \", tags.join(\" \"), \"\\n\"), \"\"), ",
    "if(root, \"\", concat(",
    "\"Author:    \", author.name(), \" <\", author.email(), \"> (\", ",
    "author.timestamp().ago(), \")\\n\", ",
    "\"Committer: \", committer.name(), \" <\", committer.email(), \"> (\", ",
    "committer.timestamp().ago(), \")\\n\")), ",
    "\"\\n\", ",
    "if(description, description, \"(no description set)\\n\"))",
);

#[derive(Debug, thiserror::Error)]
pub enum JjError {
    #[error("failed to run jj: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("jj {cmd} failed ({code}): {stderr}")]
    Failed {
        cmd: String,
        code: i32,
        stderr: String,
    },
}

#[derive(Debug, Clone)]
pub struct JjOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl JjOutput {
    pub fn ok(&self) -> bool {
        self.status == 0
    }
}

/// One executed jj command, kept for the `$` process-log buffer.
#[derive(Debug, Clone)]
pub struct ProcessEntry {
    pub cmd: String,
    pub status: i32,
    pub output: String,
}

#[derive(Debug, Clone)]
pub struct JjClient {
    /// The workspace root (where the working copy lives).
    pub workspace_root: PathBuf,
    /// The `.jj` directory, for the fs watcher.
    pub jj_dir: PathBuf,
    /// Revset for the status buffer's log section and the default log
    /// buffer; `None` uses jj's own default (`revsets.log`).
    pub log_revset: Option<String>,
}

impl JjClient {
    /// Discover the workspace containing `cwd`.
    pub fn discover(cwd: &Path) -> Result<Self, JjError> {
        let root = run_in(cwd, &["workspace", "root"], None, false)?;
        if !root.ok() {
            return Err(JjError::Failed {
                cmd: "workspace root".into(),
                code: root.status,
                stderr: root.stderr.trim().to_string(),
            });
        }
        let workspace_root = PathBuf::from(root.stdout.trim_end());
        let jj_dir = workspace_root.join(".jj");
        Ok(Self {
            workspace_root,
            jj_dir,
            log_revset: None,
        })
    }

    /// Run a jj mutation; non-zero exit is reported via `JjOutput::status`,
    /// not `Err`. Mutations snapshot the working copy themselves.
    pub fn run(&self, args: &[&str]) -> Result<JjOutput, JjError> {
        run_in(&self.workspace_root, args, None, false)
    }

    /// Run a jj read with `--ignore-working-copy` (no snapshot, no op).
    pub fn run_read(&self, args: &[&str]) -> Result<JjOutput, JjError> {
        run_in(&self.workspace_root, args, None, true)
    }

    /// Like `run_read` but turns a non-zero exit into `Err` — for reads that
    /// must succeed.
    fn read(&self, args: &[&str]) -> Result<JjOutput, JjError> {
        let out = self.run_read(args)?;
        if out.ok() {
            Ok(out)
        } else {
            Err(JjError::Failed {
                cmd: args.join(" "),
                code: out.status,
                stderr: out.stderr.trim().to_string(),
            })
        }
    }

    /// Read everything the status buffer needs. Runs on a worker thread.
    ///
    /// The working-copy read goes first *without* `--ignore-working-copy` so
    /// it snapshots external file edits; the remaining reads then see the
    /// fresh operation and run concurrently with `--ignore-working-copy`
    /// (snapshotting concurrently would contend on the repo lock).
    pub fn read_snapshot(&self) -> Result<StatusSnapshot, JjError> {
        let tmpl = log_template_no_graph();
        let wc_out = run_in(
            &self.workspace_root,
            &["log", "--no-graph", "-r", "@", "-T", &tmpl],
            None,
            false,
        )?;
        if !wc_out.ok() {
            return Err(JjError::Failed {
                cmd: "log -r @".into(),
                code: wc_out.status,
                stderr: wc_out.stderr.trim().to_string(),
            });
        }
        let working_copy = parse::parse_rev_entries(&wc_out.stdout).into_iter().next();

        let (parents, diff, log, conflicts, bookmarks) = std::thread::scope(|s| {
            let parents =
                s.spawn(|| self.read(&["log", "--no-graph", "-r", "parents(@)", "-T", &tmpl]));
            let diff = s.spawn(|| self.read(&["diff", "-r", "@", "--git"]));
            let log = s.spawn(|| {
                let mut args = vec!["log", "-T", LOG_TEMPLATE];
                if let Some(revset) = &self.log_revset {
                    args.extend(["-r", revset.as_str()]);
                }
                self.read(&args)
            });
            // Exits non-zero when there are no conflicts; treated as empty.
            let conflicts = s.spawn(|| self.run_read(&["resolve", "--list"]));
            let bookmarks = s.spawn(|| {
                self.read(&[
                    "bookmark",
                    "list",
                    "-T",
                    "concat(name, \"\\0\", if(remote, remote, \"\"), \"\\n\")",
                ])
            });
            (
                parents.join().unwrap(),
                diff.join().unwrap(),
                log.join().unwrap(),
                conflicts.join().unwrap(),
                bookmarks.join().unwrap(),
            )
        });

        let conflicts = match conflicts? {
            out if out.ok() => parse::parse_conflict_list(&out.stdout),
            _ => Vec::new(),
        };

        Ok(StatusSnapshot {
            working_copy,
            parents: parse::parse_rev_entries(&parents?.stdout),
            conflicts,
            diff: Arc::new(parse::parse_diff(&diff?.stdout)),
            log: parse::parse_log_rows(&log?.stdout),
            bookmarks: parse::parse_bookmarks(&bookmarks?.stdout),
        })
    }

    /// Read a log-like buffer (`jj log -r ...` or `jj evolog -r ...`).
    /// `args` is the full argument list before the template.
    pub fn read_log(&self, args: &[String]) -> Result<Vec<LogRow>, JjError> {
        // Evolog rows are evolution entries, not commits — they need the
        // `commit.`-prefixed template.
        let template = if args.first().is_some_and(|a| a == "evolog") {
            EVOLOG_TEMPLATE
        } else {
            LOG_TEMPLATE
        };
        let mut full: Vec<&str> = args.iter().map(String::as_str).collect();
        full.extend(["-T", template]);
        Ok(parse::parse_log_rows(&self.read(&full)?.stdout))
    }

    /// Read the header + diff of one revision for a revision buffer.
    pub fn read_revision(&self, rev: &str) -> Result<(String, String), JjError> {
        let header = self
            .read(&["log", "--no-graph", "-r", rev, "-T", SHOW_TEMPLATE])?
            .stdout;
        let diff = self.read(&["diff", "-r", rev, "--git"])?.stdout;
        Ok((header, diff))
    }

    /// Read the operation log.
    pub fn read_op_log(&self) -> Result<Vec<OpLogRow>, JjError> {
        let out = self.read(&["op", "log", "-T", OP_TEMPLATE])?;
        Ok(parse::parse_op_rows(&out.stdout))
    }

    /// Read `jj op show` for one operation (plain text).
    pub fn read_op_show(&self, op_id: &str) -> Result<String, JjError> {
        Ok(self.read(&["op", "show", op_id])?.stdout)
    }

    /// Candidates for destination/revision pickers: bookmark names plus a
    /// few useful revset symbols.
    pub fn rev_candidates(snapshot: Option<&StatusSnapshot>) -> Vec<String> {
        let mut out = vec!["@".to_string(), "@-".to_string(), "trunk()".to_string()];
        if let Some(s) = snapshot {
            for b in &s.bookmarks {
                let name = match &b.remote {
                    Some(remote) => format!("{}@{}", b.name, remote),
                    None => b.name.clone(),
                };
                if !out.contains(&name) {
                    out.push(name);
                }
            }
        }
        out
    }
}

fn run_in(
    dir: &Path,
    args: &[&str],
    stdin: Option<&str>,
    ignore_working_copy: bool,
) -> Result<JjOutput, JjError> {
    let mut cmd = Command::new("jj");
    cmd.arg("--no-pager").arg("--color=never");
    if ignore_working_copy {
        cmd.arg("--ignore-working-copy");
    }
    cmd.args(args)
        .current_dir(dir)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    if let Some(input) = stdin {
        // The child may exit without draining stdin; a write error then is fine.
        if let Some(mut pipe) = child.stdin.take() {
            let _ = pipe.write_all(input.as_bytes());
        }
    }
    let out = child.wait_with_output()?;
    Ok(JjOutput {
        status: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

/// Human-readable command line for messages and the process log.
pub fn display_cmd(args: &[String]) -> String {
    let mut s = String::from("jj");
    for a in args {
        s.push(' ');
        if a.contains(' ') {
            s.push('\'');
            s.push_str(a);
            s.push('\'');
        } else {
            s.push_str(a);
        }
    }
    s
}
