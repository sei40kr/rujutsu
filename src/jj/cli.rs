//! The `jj` command-line adapter: the only place that knows how an
//! operation spells itself as argv. Reads run with `--ignore-working-copy`
//! so they never snapshot (which would write an operation and re-trigger the
//! fs watcher); the one exception is the working-copy read at the top of
//! `snapshot`, which deliberately snapshots so external file edits show up.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use super::op::{
    Author, BookmarkOp, Checkout, Commit, Deletions, Describe, EmptiedCommits, EmptiedSource,
    Fetch, LogQuery, LogScope, Message, NewChange, OperationId, Placement, Push, PushScope, Rebase,
    RebaseKind, Restore, Revset, RunMode, Squash, SquashMessage, SquashSource,
};
use super::types::{LogRow, OpLogRow, RevisionView, StatusSnapshot};
use super::{parse, Jj, JjError, Outcome};

/// Template for one log/evolog row. Starts each record with NUL so the
/// parser can split the graph prefix from the fields, and NUL-separates the
/// fields (descriptions cannot contain NUL). The flag field packs booleans as
/// single letters; the last two fields are the lengths of the ids' unique
/// prefixes, which jj colors apart from the rest. Graph mode adds line breaks
/// itself; the `--no-graph` variant appends its own.
pub const LOG_TEMPLATE: &str = concat!(
    "concat(\"\\0\", change_id.shortest(8), \"\\0\", commit_id.shortest(8), \"\\0\", ",
    "bookmarks.join(\" \"), \"\\0\", tags.join(\" \"), \"\\0\", ",
    "if(description, description.first_line(), \"\"), \"\\0\", ",
    "author.name(), \"\\0\", committer.timestamp().ago(), \"\\0\", ",
    "if(current_working_copy, \"@\", \"\"), if(conflict, \"C\", \"\"), ",
    "if(empty, \"E\", \"\"), if(immutable, \"I\", \"\"), if(root, \"R\", \"\"), ",
    "if(divergent, \"D\", \"\"), if(hidden, \"H\", \"\"), \"\\0\", ",
    "change_id.shortest(8).prefix().len(), \"\\0\", commit_id.shortest(8).prefix().len())",
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
    "if(commit.divergent(), \"D\", \"\"), if(commit.hidden(), \"H\", \"\"), \"\\0\", ",
    "commit.change_id().shortest(8).prefix().len(), \"\\0\", ",
    "commit.commit_id().shortest(8).prefix().len())",
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

#[derive(Debug, Clone)]
struct Captured {
    status: i32,
    stdout: String,
    stderr: String,
}

impl Captured {
    fn ok(&self) -> bool {
        self.status == 0
    }
}

#[derive(Debug, Clone)]
pub struct JjCli {
    workspace_root: PathBuf,
    /// The workspace's `.jj` directory.
    pub jj_dir: PathBuf,
}

impl JjCli {
    /// Discover the workspace containing `cwd`.
    pub fn discover(cwd: &Path) -> Result<Self, JjError> {
        let root = run_in(cwd, &["workspace", "root"], false)?;
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
        })
    }

    /// Run a mutation with captured output. Mutations snapshot the working
    /// copy themselves.
    fn run(&self, args: &[&str]) -> Result<Outcome, JjError> {
        let out = run_in(&self.workspace_root, args, false)?;
        Ok(Outcome {
            cmd: display_cmd(args),
            status: out.status,
            output: format!("{}{}", out.stdout, out.stderr),
        })
    }

    /// Run a mutation with inherited stdio, so jj can drive `$EDITOR`.
    fn run_fg(&self, args: &[&str]) -> Result<Outcome, JjError> {
        let status = Command::new("jj")
            .args(args)
            .current_dir(&self.workspace_root)
            .status()?;
        Ok(Outcome {
            cmd: display_cmd(args),
            status: status.code().unwrap_or(-1),
            output: String::new(), // stdio went to the terminal
        })
    }

    /// jj moved author resets out of describe/commit into `metaedit`, so a
    /// reset runs first as its own step; `then` runs only if it succeeded.
    fn then_reset_author(
        &self,
        author: Author,
        rev: &Revset,
        then: impl FnOnce() -> Result<Outcome, JjError>,
    ) -> Result<Outcome, JjError> {
        if author == Author::Keep {
            return then();
        }
        let reset = self.run(&["metaedit", "--update-author", "-r", rev.as_str()])?;
        if !reset.ok() {
            return Ok(reset);
        }
        let main = then()?;
        Ok(Outcome {
            cmd: format!("{}; {}", reset.cmd, main.cmd),
            status: main.status,
            output: format!("{}{}", reset.output, main.output),
        })
    }

    /// Run a read with `--ignore-working-copy` (no snapshot, no op).
    fn run_read(&self, args: &[&str]) -> Result<Captured, JjError> {
        run_in(&self.workspace_root, args, true)
    }

    /// Like `run_read` but turns a non-zero exit into `Err` — for reads that
    /// must succeed.
    fn read(&self, args: &[&str]) -> Result<Captured, JjError> {
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
}

fn describe_args<'a>(op: &'a Describe, msg: Option<&'a Message>) -> Vec<&'a str> {
    let mut a = vec!["describe", op.rev.as_str()];
    if let Some(m) = msg {
        a.extend(["-m", m.as_str()]);
    }
    a
}

fn commit_args(msg: Option<&Message>) -> Vec<&str> {
    let mut a = vec!["commit"];
    if let Some(m) = msg {
        a.extend(["-m", m.as_str()]);
    }
    a
}

fn squash_args<'a>(op: &'a Squash, msg: Option<&'a SquashMessage>) -> Vec<&'a str> {
    let mut a = vec!["squash"];
    match msg {
        Some(SquashMessage::Given(m)) => a.extend(["-m", m.as_str()]),
        Some(SquashMessage::UseDestination) => a.push("--use-destination-message"),
        None => {} // jj opens $EDITOR to combine the descriptions
    }
    if op.emptied == EmptiedSource::Keep {
        a.push("--keep-emptied");
    }
    match &op.source {
        SquashSource::Revision(r) => a.extend(["-r", r.as_str()]),
        SquashSource::WorkingCopyInto(r) => a.extend(["--into", r.as_str()]),
        // Paths are positional and must come last.
        SquashSource::WorkingCopyPath(p) => a.extend(["--", p.as_str()]),
    }
    a
}

impl Jj for JjCli {
    fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// The working-copy read goes first *without* `--ignore-working-copy` so
    /// it snapshots external file edits; the remaining reads then see the
    /// fresh operation and run concurrently with `--ignore-working-copy`
    /// (snapshotting concurrently would contend on the repo lock).
    fn snapshot(&self, log_revset: Option<&Revset>) -> Result<StatusSnapshot, JjError> {
        let tmpl = log_template_no_graph();
        let wc_out = run_in(
            &self.workspace_root,
            &["log", "--no-graph", "-r", "@", "-T", &tmpl],
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
                if let Some(revset) = log_revset {
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

    fn log(&self, query: &LogQuery) -> Result<Vec<LogRow>, JjError> {
        // Evolog rows are evolution entries, not commits — they need the
        // `commit.`-prefixed template.
        let mut args = match &query.scope {
            LogScope::Default => vec!["log", "-T", LOG_TEMPLATE],
            LogScope::Revset(r) => vec!["log", "-r", r.as_str(), "-T", LOG_TEMPLATE],
            LogScope::Evolution(r) => vec!["evolog", "-r", r.as_str(), "-T", EVOLOG_TEMPLATE],
        };
        let limit = query.limit.map(|n| n.to_string());
        if let Some(n) = &limit {
            args.extend(["-n", n.as_str()]);
        }
        Ok(parse::parse_log_rows(&self.read(&args)?.stdout))
    }

    fn revision(&self, rev: &Revset) -> Result<RevisionView, JjError> {
        let header = self
            .read(&["log", "--no-graph", "-r", rev.as_str(), "-T", SHOW_TEMPLATE])?
            .stdout;
        let diff = self.read(&["diff", "-r", rev.as_str(), "--git"])?.stdout;
        Ok(RevisionView {
            header,
            files: parse::parse_diff(&diff),
        })
    }

    fn op_log(&self) -> Result<Vec<OpLogRow>, JjError> {
        let out = self.read(&["op", "log", "-T", OP_TEMPLATE])?;
        Ok(parse::parse_op_rows(&out.stdout))
    }

    fn op_show(&self, op: &OperationId) -> Result<String, JjError> {
        Ok(self.read(&["op", "show", op.as_str()])?.stdout)
    }

    fn describe(&self, op: &Describe, msg: &Message) -> Result<Outcome, JjError> {
        self.then_reset_author(op.author, &op.rev, || {
            self.run(&describe_args(op, Some(msg)))
        })
    }

    fn commit(&self, op: &Commit, msg: &Message) -> Result<Outcome, JjError> {
        let wc = Revset::working_copy();
        self.then_reset_author(op.author, &wc, || self.run(&commit_args(Some(msg))))
    }

    fn new_change(&self, op: &NewChange) -> Result<Outcome, JjError> {
        let mut a = vec!["new"];
        match op.placement {
            Placement::ChildOf => {}
            Placement::After => a.push("--insert-after"),
            Placement::Before => a.push("--insert-before"),
        }
        a.push(op.anchor.as_str());
        if op.checkout == Checkout::StayPut {
            a.push("--no-edit");
        }
        self.run(&a)
    }

    fn squash(&self, op: &Squash, msg: &SquashMessage) -> Result<Outcome, JjError> {
        self.run(&squash_args(op, Some(msg)))
    }

    fn rebase(&self, op: &Rebase) -> Result<Outcome, JjError> {
        let kind = match op.kind {
            RebaseKind::Revision => "-r",
            RebaseKind::WithDescendants => "-s",
            RebaseKind::Branch => "-b",
        };
        let mut a = vec!["rebase", kind, op.rev.as_str(), "-d", op.onto.as_str()];
        if op.emptied == EmptiedCommits::Skip {
            a.push("--skip-emptied");
        }
        self.run(&a)
    }

    fn bookmark(&self, op: &BookmarkOp) -> Result<Outcome, JjError> {
        let remote;
        let a: Vec<&str> = match op {
            BookmarkOp::Create { name, at } => {
                vec!["bookmark", "create", name.as_str(), "-r", at.as_str()]
            }
            BookmarkOp::Set { name, at } => {
                vec!["bookmark", "set", name.as_str(), "-r", at.as_str()]
            }
            BookmarkOp::Delete(name) => vec!["bookmark", "delete", name.as_str()],
            BookmarkOp::Rename { from, to } => {
                vec!["bookmark", "rename", from.as_str(), to.as_str()]
            }
            BookmarkOp::Forget(name) => vec!["bookmark", "forget", name.as_str()],
            BookmarkOp::Track(b) => {
                remote = b.to_string();
                vec!["bookmark", "track", &remote]
            }
            BookmarkOp::Untrack(b) => {
                remote = b.to_string();
                vec!["bookmark", "untrack", &remote]
            }
        };
        self.run(&a)
    }

    fn push(&self, op: &Push) -> Result<Outcome, JjError> {
        let mut a = vec!["git", "push"];
        match &op.scope {
            PushScope::Tracked => a.push("--tracked"),
            PushScope::Bookmark(b) => a.extend(["--bookmark", b.as_str()]),
            PushScope::Change(r) => a.extend(["--change", r.as_str()]),
        }
        if op.options.deletions == Deletions::Propagate {
            a.push("--deleted");
        }
        if op.options.mode == RunMode::DryRun {
            a.push("--dry-run");
        }
        self.run(&a)
    }

    fn fetch(&self, op: &Fetch) -> Result<Outcome, JjError> {
        match op {
            Fetch::DefaultRemote => self.run(&["git", "fetch"]),
            Fetch::AllRemotes => self.run(&["git", "fetch", "--all-remotes"]),
        }
    }

    fn abandon(&self, rev: &Revset) -> Result<Outcome, JjError> {
        self.run(&["abandon", "-r", rev.as_str()])
    }

    fn restore(&self, op: &Restore) -> Result<Outcome, JjError> {
        match op {
            Restore::Path(p) => self.run(&["restore", "--", p.as_str()]),
            Restore::WorkingCopy => self.run(&["restore"]),
        }
    }

    fn edit(&self, rev: &Revset) -> Result<Outcome, JjError> {
        self.run(&["edit", "-r", rev.as_str()])
    }

    fn duplicate(&self, rev: &Revset) -> Result<Outcome, JjError> {
        self.run(&["duplicate", rev.as_str()])
    }

    fn absorb(&self) -> Result<Outcome, JjError> {
        self.run(&["absorb"])
    }

    fn undo(&self) -> Result<Outcome, JjError> {
        self.run(&["undo"])
    }

    fn redo(&self) -> Result<Outcome, JjError> {
        self.run(&["redo"])
    }

    fn op_restore(&self, op: &OperationId) -> Result<Outcome, JjError> {
        self.run(&["op", "restore", op.as_str()])
    }

    fn describe_in_editor(&self, op: &Describe) -> Result<Outcome, JjError> {
        self.then_reset_author(op.author, &op.rev, || self.run_fg(&describe_args(op, None)))
    }

    fn commit_in_editor(&self, op: &Commit) -> Result<Outcome, JjError> {
        let wc = Revset::working_copy();
        self.then_reset_author(op.author, &wc, || self.run_fg(&commit_args(None)))
    }

    fn squash_in_editor(&self, op: &Squash) -> Result<Outcome, JjError> {
        self.run_fg(&squash_args(op, None))
    }

    fn split_in_editor(&self, rev: &Revset) -> Result<Outcome, JjError> {
        self.run_fg(&["split", "-r", rev.as_str()])
    }
}

fn run_in(dir: &Path, args: &[&str], ignore_working_copy: bool) -> Result<Captured, JjError> {
    let mut cmd = Command::new("jj");
    cmd.arg("--no-pager").arg("--color=never");
    if ignore_working_copy {
        cmd.arg("--ignore-working-copy");
    }
    let out = cmd
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    Ok(Captured {
        status: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

/// Human-readable command line for messages and the process log.
fn display_cmd(args: &[&str]) -> String {
    let mut s = String::from("jj");
    for a in args {
        s.push(' ');
        if a.contains(' ') || a.is_empty() {
            s.push('\'');
            s.push_str(a);
            s.push('\'');
        } else {
            s.push_str(a);
        }
    }
    s
}
