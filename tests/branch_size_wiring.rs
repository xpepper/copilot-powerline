//! Runs the built binary against a throwaway `HOME` that is a real git
//! repository, with a local bare repository as `origin`, to check the
//! `branch_size` wiring end to end: a refresh with no cache must spawn the
//! `--fetch-branch-size` worker, the worker must measure the branch with real
//! `git`, and the next refresh must show what it cached. The unit tests in
//! `src/branch_size.rs` and `src/segments/branch_size.rs` cover parsing, the
//! cache rules and rendering.
//!
//! A `git` wrapper on `PATH` logs the command line of the process that ran
//! each call, so a test can tell the worker's calls from the refresh's. It
//! hangs instead while a `git-hangs` file exists in `HOME`.
#![cfg(unix)]

mod common;

use common::{SESSION, SESSION_SPEND, Sandbox, payload};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const SHOWN: &str = "theme = \"plain\"\nsegments = [\"branch_size\"]\n";

fn real_git() -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|dir| dir.join("git"))
        .find(|candidate| candidate.is_file())
        .expect("git must be installed to run these tests")
}

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new(real_git())
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("HOME", dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn commit_file(dir: &Path, name: &str, content: &str) {
    fs::write(dir.join(name), content).unwrap();
    git(dir, &["add", name]);
    git(dir, &["commit", "-q", "-m", name]);
}

/// Makes `dir` a repository on `main` with one commit, whose `origin` is
/// the bare repository `origin` with `main` as its default branch.
fn init_with_origin(dir: &Path, origin: &Path) {
    git(origin, &["init", "-q", "--bare", "-b", "main"]);
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "user.name", "Test"]);
    git(dir, &["config", "user.email", "test@example.com"]);
    commit_file(dir, "a.txt", "one\ntwo\n");
    git(dir, &["remote", "add", "origin", origin.to_str().unwrap()]);
    git(dir, &["push", "-q", "origin", "main"]);
    git(dir, &["remote", "set-head", "origin", "main"]);
}

/// The sandbox `HOME` as a checkout on branch `feature`, whose `origin` is
/// a bare repository with `main` as its default branch.
struct Repo {
    sandbox: Sandbox,
    _dirs: Vec<TempDir>,
    bin: PathBuf,
    log: PathBuf,
    hang: PathBuf,
}

impl Repo {
    /// `HOME` is the repository itself.
    fn new() -> Self {
        let sandbox = Sandbox::new();
        let origin = tempfile::tempdir().unwrap();
        init_with_origin(sandbox.home(), origin.path());
        git(sandbox.home(), &["switch", "-q", "-c", "feature"]);
        Self::with_git_wrapper(sandbox, vec![origin])
    }

    /// `HOME` is a linked worktree of a repository that lives elsewhere.
    fn linked_worktree() -> Self {
        let sandbox = Sandbox::new();
        let origin = tempfile::tempdir().unwrap();
        let main = tempfile::tempdir().unwrap();
        init_with_origin(main.path(), origin.path());
        git(
            main.path(),
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feature",
                sandbox.home().to_str().unwrap(),
            ],
        );
        Self::with_git_wrapper(sandbox, vec![origin, main])
    }

    fn with_git_wrapper(sandbox: Sandbox, dirs: Vec<TempDir>) -> Self {
        let home = sandbox.home().to_path_buf();
        let bin = home.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let log = home.join("git-calls.log");
        let hang = home.join("git-hangs");
        let wrapper = bin.join("git");
        fs::write(
            &wrapper,
            format!(
                "#!/bin/sh\n\
                 [ -f '{hang}' ] && exec /bin/sleep 60\n\
                 /bin/ps -o args= -p \"$PPID\" >> '{log}'\n\
                 exec '{git}' \"$@\"\n",
                hang = hang.display(),
                log = log.display(),
                git = real_git().display(),
            ),
        )
        .unwrap();
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();

        Self {
            sandbox,
            _dirs: dirs,
            bin,
            log,
            hang,
        }
    }

    fn home(&self) -> &Path {
        self.sandbox.home()
    }

    /// Pushes a commit to `origin/main` that the branch does not have, as
    /// when another pull request merges after the branch started.
    fn advance_origin_main(&self) {
        git(self.home(), &["switch", "-q", "-c", "other", "main"]);
        commit_file(self.home(), "late.txt", "late\n");
        git(self.home(), &["push", "-q", "origin", "other:main"]);
        git(self.home(), &["switch", "-q", "feature"]);
        git(self.home(), &["branch", "-q", "-D", "other"]);
    }

    fn refresh(&self) -> String {
        self.sandbox.refresh_with_env(
            &payload(SESSION, SESSION_SPEND),
            &["--style", "plain", "--icon-set", "plain"],
            &[("PATH", &self.bin)],
        )
    }

    /// Refreshes with a `PATH` on which there is no `git`.
    fn refresh_without_git(&self) -> String {
        let empty = self.home().join("empty-bin");
        fs::create_dir_all(&empty).unwrap();
        self.sandbox.refresh_with_env(
            &payload(SESSION, SESSION_SPEND),
            &["--style", "plain", "--icon-set", "plain"],
            &[("PATH", &empty)],
        )
    }

    fn wait_for_worker(&self, caches: usize) {
        self.sandbox.wait_for_worker("branch_size_", caches);
    }

    /// Refreshes, waits for the worker that refresh started, and refreshes
    /// again to show what it cached.
    fn shown_after_fetch(&self, caches: usize) -> String {
        self.refresh();
        self.wait_for_worker(caches);
        self.refresh()
    }

    /// Configures, adds a 3-line commit to the feature branch, and shows the
    /// segment once the worker has cached its numbers.
    fn shown_with_config_after_commit(&self, config: &str) -> String {
        self.sandbox.write_config(config);
        commit_file(self.home(), "c.txt", "x\ny\nz\n");
        self.shown_after_fetch(1)
    }

    /// Like `shown_with_config_after_commit`, with one changed line left
    /// uncommitted in `a.txt`.
    fn shown_with_config_after_commit_and_edit(&self, config: &str) -> String {
        fs::write(self.home().join("a.txt"), "one\nTWO\n").unwrap();
        self.shown_with_config_after_commit(config)
    }

    /// The command line of the process behind each `git` call so far.
    fn git_callers(&self) -> Vec<String> {
        fs::read_to_string(&self.log)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }
}

#[test]
fn test_shows_the_committed_size_after_the_background_refresh() {
    let repo = Repo::new();
    repo.sandbox.write_config(SHOWN);
    commit_file(repo.home(), "a.txt", "one\nTWO\nthree\n");
    commit_file(repo.home(), "c.txt", "x\ny\nz\n");
    repo.advance_origin_main();

    let before = repo.refresh();
    repo.wait_for_worker(1);
    let after = repo.refresh();

    assert_eq!(before, "");
    assert_eq!(after, "+5 -1 · 2 files vs main");
    let callers = repo.git_callers();
    assert!(!callers.is_empty(), "the worker ran no git");
    assert!(
        callers.iter().all(|c| c.contains("--fetch-branch-size")),
        "git ran outside the worker: {callers:?}"
    );
}

#[test]
fn test_adds_staged_and_unstaged_changes_as_uncommitted() {
    let repo = Repo::new();
    repo.sandbox.write_config(SHOWN);
    commit_file(repo.home(), "c.txt", "x\ny\nz\n");
    fs::write(repo.home().join("a.txt"), "one\nTWO\n").unwrap();
    fs::write(repo.home().join("d.txt"), "d\ne\n").unwrap();
    git(repo.home(), &["add", "d.txt"]);

    let line = repo.shown_after_fetch(1);

    assert_eq!(line, "+3 -0 · 1 file vs main · +3 -1 uncommitted");
}

#[test]
fn test_only_uncommitted_changes_show_an_empty_committed_part() {
    let repo = Repo::new();
    repo.sandbox.write_config(SHOWN);
    fs::write(repo.home().join("a.txt"), "one\nTWO\n").unwrap();

    let line = repo.shown_after_fetch(1);

    assert_eq!(line, "+0 -0 · 0 files vs main · +1 -1 uncommitted");
}

#[test]
fn test_a_file_named_head_is_not_mistaken_for_the_head_commit() {
    let repo = Repo::new();
    repo.sandbox.write_config(SHOWN);
    commit_file(repo.home(), "HEAD", "h\n");
    fs::write(repo.home().join("a.txt"), "one\nTWO\n").unwrap();

    let line = repo.shown_after_fetch(1);

    assert_eq!(line, "+1 -0 · 1 file vs main · +1 -1 uncommitted");
}

#[test]
fn test_each_branch_shows_its_own_size() {
    let repo = Repo::new();
    repo.sandbox.write_config(SHOWN);
    commit_file(repo.home(), "c.txt", "x\ny\nz\n");
    repo.shown_after_fetch(1);
    git(repo.home(), &["switch", "-q", "-c", "second", "main"]);
    commit_file(repo.home(), "d.txt", "d\n");

    let before = repo.refresh();
    repo.wait_for_worker(2);
    let after = repo.refresh();

    assert_eq!(before, "");
    assert_eq!(after, "+1 -0 · 1 file vs main");
}

#[test]
fn test_a_fresh_size_is_shown_without_measuring_again() {
    let repo = Repo::new();
    repo.sandbox.write_config(&format!(
        "{SHOWN}\n[branch_size]\ncache_ttl_seconds = 3600\n"
    ));
    commit_file(repo.home(), "c.txt", "x\ny\nz\n");
    repo.shown_after_fetch(1);
    let calls = repo.git_callers().len();
    commit_file(repo.home(), "d.txt", "d\n");

    let line = repo.refresh();

    assert_eq!(line, "+3 -0 · 1 file vs main");
    assert_eq!(repo.git_callers().len(), calls, "measured again");
}

#[test]
fn test_a_stale_size_is_shown_until_the_next_refresh_replaces_it() {
    let repo = Repo::new();
    let with_ttl = |seconds: u64| {
        repo.sandbox.write_config(&format!(
            "{SHOWN}\n[branch_size]\ncache_ttl_seconds = {seconds}\n"
        ))
    };
    with_ttl(3600);
    commit_file(repo.home(), "c.txt", "x\ny\nz\n");
    repo.shown_after_fetch(1);
    commit_file(repo.home(), "d.txt", "d\n");
    with_ttl(0);

    let stale = repo.refresh();
    repo.wait_for_worker(1);
    let replaced = repo.refresh();

    assert_eq!(stale, "+3 -0 · 1 file vs main");
    assert_eq!(replaced, "+4 -0 · 2 files vs main");
}

/// `WORKER_TIMEOUT` in `src/worker.rs` is 10 s, and the throttle lets the
/// next worker start 15 s after the previous one.
const THROTTLE: Duration = Duration::from_secs(15);

#[test]
fn test_a_hung_git_is_stopped_before_the_next_refresh_may_start() {
    let repo = Repo::new();
    repo.sandbox.write_config(SHOWN);
    commit_file(repo.home(), "c.txt", "x\ny\nz\n");
    fs::write(&repo.hang, "").unwrap();

    let started = Instant::now();
    repo.refresh();
    repo.wait_for_worker(1);
    let elapsed = started.elapsed();

    assert!(elapsed < THROTTLE, "the worker ran for {elapsed:?}");
    assert_eq!(repo.refresh(), "");
}

#[test]
fn test_compares_with_the_configured_default_branch() {
    let repo = Repo::new();
    repo.sandbox.write_config(&format!(
        "{SHOWN}\n[branch_size]\ndefault_branch = \"develop\"\n"
    ));
    git(repo.home(), &["switch", "-q", "-c", "develop", "main"]);
    commit_file(repo.home(), "c.txt", "x\ny\nz\n");
    git(repo.home(), &["push", "-q", "origin", "develop"]);
    git(repo.home(), &["switch", "-q", "-c", "topic"]);
    commit_file(repo.home(), "d.txt", "d\n");

    let line = repo.shown_after_fetch(1);

    assert_eq!(line, "+1 -0 · 1 file vs develop");
}

#[test]
fn test_a_local_branch_named_like_the_remote_one_is_ignored() {
    let repo = Repo::new();
    git(repo.home(), &["branch", "origin/main"]);

    let line = repo.shown_with_config_after_commit(SHOWN);

    assert_eq!(line, "+3 -0 · 1 file vs main");
}

#[test]
fn test_a_local_branch_named_like_the_configured_remote_one_is_ignored() {
    let repo = Repo::new();
    git(repo.home(), &["branch", "origin/main"]);

    let line = repo.shown_with_config_after_commit(&format!(
        "{SHOWN}\n[branch_size]\ndefault_branch = \"main\"\n"
    ));

    assert_eq!(line, "+3 -0 · 1 file vs main");
}

#[test]
fn test_reads_the_default_branch_in_a_linked_worktree() {
    let repo = Repo::linked_worktree();
    repo.sandbox.write_config(SHOWN);
    commit_file(repo.home(), "c.txt", "x\ny\nz\n");

    let line = repo.shown_after_fetch(1);

    assert_eq!(line, "+3 -0 · 1 file vs main");
}

#[test]
fn test_hidden_without_a_default_branch() {
    let repo = Repo::new();
    repo.sandbox.write_config(SHOWN);
    commit_file(repo.home(), "c.txt", "x\ny\nz\n");
    git(repo.home(), &["remote", "set-head", "origin", "--delete"]);

    let line = repo.shown_after_fetch(1);

    assert_eq!(line, "");
}

#[test]
fn test_hidden_on_the_default_branch() {
    let repo = Repo::new();
    git(repo.home(), &["switch", "-q", "main"]);

    let line = repo.shown_with_config_after_commit_and_edit(SHOWN);

    assert_eq!(line, "");
}

#[test]
fn test_hidden_on_a_detached_head() {
    let repo = Repo::new();
    repo.sandbox.write_config(SHOWN);
    commit_file(repo.home(), "c.txt", "x\ny\nz\n");
    git(repo.home(), &["switch", "-q", "--detach"]);

    let line = repo.refresh();

    assert_eq!(line, "");
    assert!(repo.sandbox.state_files("branch_size_").is_empty());
}

#[test]
fn test_hidden_without_changes() {
    let repo = Repo::new();
    repo.sandbox.write_config(SHOWN);

    let line = repo.shown_after_fetch(1);

    assert_eq!(line, "");
}

#[test]
fn test_hidden_outside_a_repository() {
    let sandbox = Sandbox::new();
    sandbox.write_config(SHOWN);

    let line = sandbox.refresh_plain();

    assert_eq!(line, "");
    assert!(sandbox.state_files("branch_size_").is_empty());
}

#[test]
fn test_hidden_without_git_and_never_starts_a_worker() {
    let repo = Repo::new();
    repo.sandbox.write_config(SHOWN);
    commit_file(repo.home(), "c.txt", "x\ny\nz\n");

    let line = repo.refresh_without_git();

    assert_eq!(line, "");
    assert!(repo.sandbox.state_files("branch_size_").is_empty());
}

#[test]
fn test_hidden_when_the_default_branch_is_not_on_origin() {
    let repo = Repo::new();
    let line = repo.shown_with_config_after_commit_and_edit(&format!(
        "{SHOWN}\n[branch_size]\ndefault_branch = \"develop\"\n"
    ));

    assert_eq!(line, "");
}

#[test]
fn test_shows_the_alert_icon_above_the_line_limit() {
    let repo = Repo::new();
    repo.sandbox.write_config(
        "theme = \"github\"\nsegments = [\"branch_size\"]\n\n[branch_size]\nalert_above_lines = 3\n",
    );
    commit_file(repo.home(), "c.txt", "x\ny\nz\n");
    fs::write(repo.home().join("a.txt"), "one\ntwo\nthree\n").unwrap();

    let line = repo.shown_after_fetch(1);

    assert_eq!(
        line,
        "🐘 \x1b[92m+3\x1b[0m \x1b[91m-0\x1b[0m · 1 file vs main · \
         \x1b[92m+1\x1b[0m \x1b[91m-0\x1b[0m uncommitted"
    );
}

#[test]
fn test_shows_the_configured_alert_icon() {
    let repo = Repo::new();
    let line = repo.shown_with_config_after_commit(
        "theme = \"plain\"\nsegments = [\"branch_size\"]\n\n[branch_size]\nalert_above_lines = 2\nalert_icon = \"BIG \"\n",
    );

    assert_eq!(line, "BIG +3 -0 · 1 file vs main");
}

#[test]
fn test_colors_the_added_and_removed_lines() {
    let repo = Repo::new();
    let line = repo.shown_with_config_after_commit_and_edit(
        "theme = \"github\"\nsegments = [\"branch_size\"]\n",
    );

    assert_eq!(
        line,
        "\x1b[92m+3\x1b[0m \x1b[91m-0\x1b[0m · 1 file vs main · \
         \x1b[92m+1\x1b[0m \x1b[91m-1\x1b[0m uncommitted"
    );
}

#[test]
fn test_the_branch_lookup_starts_no_pr_worker() {
    let repo = Repo::new();
    repo.sandbox.write_config(SHOWN);
    commit_file(repo.home(), "c.txt", "x\ny\nz\n");
    let gh = repo.bin.join("gh");
    fs::write(&gh, "#!/bin/sh\nexit 1\n").unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();

    let line = repo.shown_after_fetch(1);

    assert_eq!(line, "+3 -0 · 1 file vs main");
    assert!(repo.sandbox.state_files("pr_").is_empty());
}
