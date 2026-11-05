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

/// The sandbox `HOME` as a repository on branch `feature`, whose `origin` is
/// a bare repository with `main` as its default branch.
struct Repo {
    sandbox: Sandbox,
    _origin: TempDir,
    bin: PathBuf,
    log: PathBuf,
    hang: PathBuf,
}

impl Repo {
    fn new() -> Self {
        let sandbox = Sandbox::new();
        let home = sandbox.home().to_path_buf();
        let origin = tempfile::tempdir().unwrap();
        git(origin.path(), &["init", "-q", "--bare", "-b", "main"]);

        git(&home, &["init", "-q", "-b", "main"]);
        git(&home, &["config", "user.name", "Test"]);
        git(&home, &["config", "user.email", "test@example.com"]);
        commit_file(&home, "a.txt", "one\ntwo\n");
        git(
            &home,
            &["remote", "add", "origin", origin.path().to_str().unwrap()],
        );
        git(&home, &["push", "-q", "origin", "main"]);
        git(&home, &["remote", "set-head", "origin", "main"]);
        git(&home, &["switch", "-q", "-c", "feature"]);

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
            _origin: origin,
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

    /// Waits for the detached worker a refresh spawned: done once `caches`
    /// branch sizes are cached and no claim lock is held.
    fn wait_for_worker(&self, caches: usize) {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let files = self.sandbox.state_files("branch_size_");
            let cached = files
                .iter()
                .filter(|p| p.extension().is_some_and(|e| e == "json"))
                .count();
            let claimed = files
                .iter()
                .any(|p| p.extension().is_some_and(|e| e == "lock"));
            if cached == caches && !claimed {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "the branch size worker never finished"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Refreshes, waits for the worker that refresh started, and refreshes
    /// again to show what it cached.
    fn shown_after_fetch(&self, caches: usize) -> String {
        self.refresh();
        self.wait_for_worker(caches);
        self.refresh()
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
    assert_eq!(after, "+1 -0 · 1 files vs main");
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

    assert_eq!(line, "+3 -0 · 1 files vs main");
    assert_eq!(repo.git_callers().len(), calls, "measured again");
}

#[test]
fn test_a_stale_size_is_shown_until_the_next_refresh_replaces_it() {
    let repo = Repo::new();
    repo.sandbox
        .write_config(&format!("{SHOWN}\n[branch_size]\ncache_ttl_seconds = 0\n"));
    commit_file(repo.home(), "c.txt", "x\ny\nz\n");
    repo.shown_after_fetch(1);
    commit_file(repo.home(), "d.txt", "d\n");

    let stale = repo.refresh();
    repo.wait_for_worker(1);
    let replaced = repo.refresh();

    assert_eq!(stale, "+3 -0 · 1 files vs main");
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
