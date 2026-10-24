use std::fs;
use std::path::Path;
use std::process::Command;

use rujutsu::jj::cli::JjCli;
use tempfile::TempDir;

/// A throwaway jj repo described by chaining steps, each one a jj subcommand
/// or a file write, in the order they happen. Also the oracle: its readers
/// go straight to `jj`, independently of the adapter under test.
pub struct Repo {
    dir: TempDir,
}

impl Repo {
    /// Panics when `jj` is not installed: a missing binary must fail the
    /// suite, not quietly turn it into a no-op.
    pub fn init() -> Self {
        let installed = Command::new("jj")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success());
        assert!(
            installed,
            "the integration tests need a `jj` binary on PATH"
        );
        let repo = Repo {
            dir: tempfile::tempdir().unwrap(),
        };
        // Repo-level user config, so the adapter's own processes (which
        // get no env from here) have an author too.
        repo.jj(&["git", "init"])
            .jj(&["config", "set", "--repo", "user.name", "Test User"])
            .jj(&["config", "set", "--repo", "user.email", "test@example.com"])
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// The adapter under test, pointed at this repo.
    pub fn cli(&self) -> JjCli {
        JjCli::discover(self.dir.path()).unwrap()
    }

    // ---- steps ----

    /// Write (or overwrite) a file in the working copy.
    pub fn file(self, path: &str, content: &str) -> Self {
        fs::write(self.dir.path().join(path), content).unwrap();
        self
    }

    pub fn describe(self, msg: &str) -> Self {
        self.jj(&["describe", "-m", msg])
    }

    pub fn bookmark(self, name: &str) -> Self {
        self.jj(&["bookmark", "create", name, "-r", "@"])
    }

    pub fn new_change(self, msg: &str) -> Self {
        self.jj(&["new", "-m", msg])
    }

    /// New change on top of `parent` (a revset).
    pub fn new_change_on(self, parent: &str, msg: &str) -> Self {
        self.jj(&["new", parent, "-m", msg])
    }

    /// Use another repo's git store as the remote `origin`. `jj git init`
    /// colocates, so the store is the `.git` next to the workspace.
    pub fn remote(self, remote: &Repo) -> Self {
        let store = remote.dir.path().join(".git");
        // Git refuses pushes to the checked-out branch of a non-bare repo;
        // the remote's working tree is never looked at, so allow it.
        let config = store.join("config");
        let mut text = fs::read_to_string(&config).unwrap();
        text.push_str("[receive]\n\tdenyCurrentBranch = ignore\n\tdenyDeleteCurrent = ignore\n");
        fs::write(&config, text).unwrap();
        self.jj(&["git", "remote", "add", "origin", store.to_str().unwrap()])
    }

    /// Run any jj subcommand; panics if it fails.
    pub fn jj(self, args: &[&str]) -> Self {
        let out = self.run(args);
        assert!(
            out.status.success(),
            "jj {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        self
    }

    // ---- oracle ----

    /// `jj log -r REV --no-graph -T TEMPLATE`, trimmed.
    pub fn show(&self, rev: &str, template: &str) -> String {
        self.out(&["log", "--no-graph", "-r", rev, "-T", template])
            .trim()
            .to_string()
    }

    /// First description lines of `::`, newest first; the root is `""`.
    pub fn subjects(&self) -> Vec<String> {
        self.out(&[
            "log",
            "--no-graph",
            "-r",
            "::",
            "-T",
            "description.first_line() ++ \"\\n\"",
        ])
        .lines()
        .map(str::to_string)
        .collect()
    }

    /// Bookmarks as `name` or `name@remote` (`*` appended when tracked),
    /// without the colocated `@git` pseudo-remote.
    pub fn bookmarks(&self) -> Vec<String> {
        self.out(&[
            "bookmark",
            "list",
            "--all-remotes",
            "-T",
            "name ++ if(remote, \"@\" ++ remote ++ if(tracked, \"*\", \"\"), \"\") ++ \"\\n\"",
        ])
        .lines()
        .filter(|l| !l.ends_with("@git") && !l.ends_with("@git*"))
        .map(str::to_string)
        .collect()
    }

    /// Paths touched by `rev`, from `jj diff --summary`.
    pub fn changed_paths(&self, rev: &str) -> Vec<String> {
        self.out(&["diff", "-r", rev, "--summary"])
            .lines()
            .map(|l| l[2..].to_string())
            .collect()
    }

    pub fn read(&self, path: &str) -> String {
        fs::read_to_string(self.dir.path().join(path)).unwrap()
    }

    fn out(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(
            out.status.success(),
            "jj {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new("jj")
            .args(args)
            .current_dir(self.dir.path())
            .env("JJ_USER", "Test User")
            .env("JJ_EMAIL", "test@example.com")
            .output()
            .expect("failed to run jj")
    }
}

/// `Revset::parse` for literals the test knows are valid.
pub fn rev(s: &str) -> rujutsu::jj::op::Revset {
    rujutsu::jj::op::Revset::parse(s).unwrap()
}

pub fn msg(s: &str) -> rujutsu::jj::op::Message {
    rujutsu::jj::op::Message::new(s)
}
