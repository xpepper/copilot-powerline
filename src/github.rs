use crate::config::PrConfig;
use crate::worker::{WORKER_TIMEOUT, lock_file_path, output_with_timeout, refresh_if_stale};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The verdict a pull request's reviews add up to (see `GLOSSARY.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewState {
    Draft,
    Approved,
    ChangesRequested,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestInfo {
    pub number: u64,
    pub url: String,
    /// Optional so cache files written before review support still parse.
    #[serde(default)]
    pub review_state: Option<ReviewState>,
    /// None when unknown: not fetched, the query failed, or the PR is not open.
    #[serde(default)]
    pub unresolved_threads: Option<UnresolvedThreads>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrCacheEntry {
    pub timestamp: u64,
    pub pr: Option<PullRequestInfo>,
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn is_gh_available() -> bool {
    crate::worker::is_on_path("gh")
}

pub fn get_cache_file_path(repo_dir: &Path, branch: &str) -> PathBuf {
    crate::state::branch_file("pr", repo_dir, branch)
}

pub fn read_cache_entry(cache_path: &Path) -> Option<PrCacheEntry> {
    let content = fs::read_to_string(cache_path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn write_cache_entry(cache_path: &Path, entry: &PrCacheEntry) -> std::io::Result<()> {
    let json = serde_json::to_string(entry)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    crate::state::write_private_atomic(cache_path, json.as_bytes())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhPrViewPayload {
    number: u64,
    url: String,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    is_draft: bool,
    // Empty when the repo requires no approving review, hence the fallback
    // to `latest_reviews`.
    #[serde(default)]
    review_decision: Option<String>,
    #[serde(default)]
    latest_reviews: Vec<GhReview>,
}

#[derive(Deserialize)]
struct GhReview {
    #[serde(default)]
    state: Option<String>,
}

/// What the worker learns from one `gh pr view` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrView {
    pub pr: PullRequestInfo,
    pub open: bool,
}

/// Parses the stdout of `gh pr view --json number,url,state,isDraft,
/// reviewDecision,latestReviews`.
pub fn parse_pr_view(stdout: &[u8]) -> Option<PrView> {
    let payload: GhPrViewPayload = serde_json::from_slice(stdout).ok()?;
    // Without a `state` (an older `gh`), the PR counts as not open: review
    // info is hidden rather than shown for a PR that may be merged.
    let open = payload.state.as_deref() == Some("OPEN");
    let review_state = if open { review_state(&payload) } else { None };

    Some(PrView {
        open,
        pr: PullRequestInfo {
            number: payload.number,
            url: payload.url,
            review_state,
            unresolved_threads: None,
        },
    })
}

/// An open PR's Review State, first match wins: draft, then GitHub's own
/// review decision, then the latest review of each reviewer.
fn review_state(payload: &GhPrViewPayload) -> Option<ReviewState> {
    if payload.is_draft {
        return Some(ReviewState::Draft);
    }
    match payload.review_decision.as_deref() {
        Some("APPROVED") => return Some(ReviewState::Approved),
        Some("CHANGES_REQUESTED") => return Some(ReviewState::ChangesRequested),
        Some("REVIEW_REQUIRED") => return None,
        _ => {}
    }
    let has_review = |wanted: &str| {
        payload
            .latest_reviews
            .iter()
            .any(|r| r.state.as_deref() == Some(wanted))
    };
    if has_review("CHANGES_REQUESTED") {
        Some(ReviewState::ChangesRequested)
    } else if has_review("APPROVED") {
        Some(ReviewState::Approved)
    } else {
        None
    }
}

/// How many review threads the GraphQL query fetches; past that the count
/// shows as a lower bound (`3+`) instead of paginating.
const REVIEW_THREADS_PAGE: u32 = 100;

const REVIEW_THREADS_QUERY: &str = r"
query($owner: String!, $repo: String!, $number: Int!, $first: Int!) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $number) {
      reviewThreads(first: $first) {
        pageInfo { hasNextPage }
        nodes { isResolved }
      }
    }
  }
}";

/// The pull request's Unresolved Threads. `more` is set when the first
/// page of threads did not hold them all, so `count` is a lower bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnresolvedThreads {
    pub count: u32,
    pub more: bool,
}

/// Parses the stdout of the `reviewThreads` GraphQL query. Outdated threads
/// count too: branch protection blocks the merge on them all the same.
pub fn parse_unresolved_threads(stdout: &[u8]) -> Option<UnresolvedThreads> {
    let json: serde_json::Value = serde_json::from_slice(stdout).ok()?;
    let threads = json.pointer("/data/repository/pullRequest/reviewThreads")?;
    let count = threads
        .get("nodes")?
        .as_array()?
        .iter()
        .filter(|t| t.get("isResolved").and_then(|r| r.as_bool()) == Some(false))
        .count();
    let more = threads
        .pointer("/pageInfo/hasNextPage")
        .and_then(|n| n.as_bool())
        .unwrap_or(false);

    Some(UnresolvedThreads {
        count: u32::try_from(count).ok()?,
        more,
    })
}

/// The owner, repo and number in a `https://github.com/<owner>/<repo>/pull/<n>`
/// URL. The URL names the PR's base repo, so a branch pushed to a fork still
/// queries the repo the PR lives in.
fn pr_coordinates(url: &str) -> Option<(&str, &str, u64)> {
    let path = url.strip_prefix("https://github.com/")?;
    match path.split('/').collect::<Vec<_>>()[..] {
        [owner, repo, "pull", number] if !owner.is_empty() && !repo.is_empty() => {
            Some((owner, repo, number.parse().ok()?))
        }
        _ => None,
    }
}

/// Runs `gh pr view` synchronously and writes the result to the cache file.
///
/// This blocks on a network call, so it must only ever run in the detached
/// background worker process spawned by `spawn_background_fetch`, never on
/// the status-line's hot path.
pub fn fetch_and_write_pr_cache(repo_dir: &Path, cache_path: &Path, fetch_unresolved: bool) {
    let now = current_timestamp();
    let started = Instant::now();

    let output = output_with_timeout(
        Command::new("gh")
            .args([
                "pr",
                "view",
                "--json",
                "number,url,state,isDraft,reviewDecision,latestReviews",
            ])
            .current_dir(repo_dir),
        WORKER_TIMEOUT,
    );

    let view = output
        .filter(|out| out.status.success())
        .and_then(|out| parse_pr_view(&out.stdout));
    let pr = view.map(|view| {
        let mut pr = view.pr;
        if view.open && fetch_unresolved {
            // Both calls share one WORKER_TIMEOUT, so the worker as a whole
            // still ends before the throttle lets the next one start.
            let remaining = WORKER_TIMEOUT.saturating_sub(started.elapsed());
            pr.unresolved_threads = fetch_unresolved_threads(repo_dir, &pr.url, remaining);
        }
        pr
    });
    let entry = PrCacheEntry { timestamp: now, pr };

    let _ = write_cache_entry(cache_path, &entry);
    let _ = fs::remove_file(lock_file_path(cache_path));
}

/// Counts the PR's Unresolved Threads with `gh api graphql`, since
/// `gh pr view --json` has no review threads field.
fn fetch_unresolved_threads(
    repo_dir: &Path,
    url: &str,
    timeout: Duration,
) -> Option<UnresolvedThreads> {
    let (owner, repo, number) = pr_coordinates(url)?;
    let output = output_with_timeout(
        Command::new("gh")
            .args(["api", "graphql", "-f"])
            .arg(format!("query={REVIEW_THREADS_QUERY}"))
            .arg("-f")
            .arg(format!("owner={owner}"))
            .arg("-f")
            .arg(format!("repo={repo}"))
            .arg("-F")
            .arg(format!("number={number}"))
            .arg("-F")
            .arg(format!("first={REVIEW_THREADS_PAGE}"))
            .current_dir(repo_dir),
        timeout,
    )?;
    output
        .status
        .success()
        .then(|| parse_unresolved_threads(&output.stdout))
        .flatten()
}

fn spawn_background_fetch(repo_dir: &Path, cache_path: &Path, fetch_unresolved: bool) {
    let mut args = vec![
        OsStr::new("--fetch-pr-cache"),
        cache_path.as_os_str(),
        OsStr::new("--repo-dir"),
        repo_dir.as_os_str(),
    ];
    if fetch_unresolved {
        args.push(OsStr::new("--fetch-unresolved"));
    }
    crate::worker::spawn_detached(&args);
}

/// Returns the current branch's PR info, if any is cached and fresh.
///
/// Never blocks on the network: a stale or missing cache triggers a
/// throttled, detached background refresh (via re-invoking this binary with
/// `--fetch-pr-cache`) and returns immediately with whatever was cached.
pub fn get_pr_info(
    repo_dir: &Path,
    branch: &str,
    config: &PrConfig,
    gh_available: bool,
) -> Option<PullRequestInfo> {
    if !gh_available {
        return None;
    }

    let cache_path = get_cache_file_path(repo_dir, branch);
    let entry = read_cache_entry(&cache_path);
    let cached_at = entry.as_ref().map(|e| e.timestamp);
    refresh_if_stale(
        &cache_path,
        cached_at,
        config.cache_ttl_seconds,
        current_timestamp(),
        |path| spawn_background_fetch(repo_dir, path, config.show_unresolved),
    );

    entry.and_then(|e| e.pr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_roundtrip() {
        let temp_file = std::env::temp_dir().join(format!(
            "test_cache_{}_{}.json",
            std::process::id(),
            current_timestamp()
        ));

        let entry = PrCacheEntry {
            timestamp: 1000,
            pr: Some(PullRequestInfo {
                number: 50,
                url: "https://github.com/org/repo/pull/50".to_string(),
                review_state: None,
                unresolved_threads: None,
            }),
        };

        write_cache_entry(&temp_file, &entry).unwrap();
        let loaded = read_cache_entry(&temp_file).unwrap();
        assert_eq!(loaded, entry);

        let _ = fs::remove_file(&temp_file);
    }

    #[test]
    fn test_cache_entry_none_roundtrip() {
        let temp_file = std::env::temp_dir().join(format!(
            "test_cache_none_{}_{}.json",
            std::process::id(),
            current_timestamp()
        ));

        let entry = PrCacheEntry {
            timestamp: 12345,
            pr: None,
        };

        write_cache_entry(&temp_file, &entry).unwrap();
        let loaded = read_cache_entry(&temp_file).unwrap();
        assert_eq!(loaded, entry);

        let _ = fs::remove_file(&temp_file);
    }

    fn pr_view(fields: &str) -> Option<PrView> {
        let json = format!(
            r#"{{"number":50,"url":"https://github.com/org/repo/pull/50","state":"OPEN",{fields}}}"#
        );
        parse_pr_view(json.as_bytes())
    }

    fn review(state: &str) -> String {
        format!(r#"{{"author":{{"login":"someone"}},"state":"{state}"}}"#)
    }

    #[test]
    fn test_changes_requested_review_beats_an_approval() {
        let view = pr_view(&format!(
            r#""isDraft":false,"reviewDecision":"","latestReviews":[{},{}]"#,
            review("APPROVED"),
            review("CHANGES_REQUESTED")
        ))
        .unwrap();

        assert_eq!(view.pr.number, 50);
        assert_eq!(view.pr.url, "https://github.com/org/repo/pull/50");
        assert_eq!(view.pr.review_state, Some(ReviewState::ChangesRequested));
        assert!(view.open);
    }

    #[test]
    fn test_a_draft_shows_as_draft_whatever_its_reviews_say() {
        let view = pr_view(&format!(
            r#""isDraft":true,"reviewDecision":"CHANGES_REQUESTED","latestReviews":[{}]"#,
            review("CHANGES_REQUESTED")
        ))
        .unwrap();

        assert_eq!(view.pr.review_state, Some(ReviewState::Draft));
    }

    #[test]
    fn test_the_review_decision_wins_over_the_latest_reviews() {
        let view = pr_view(&format!(
            r#""isDraft":false,"reviewDecision":"APPROVED","latestReviews":[{}]"#,
            review("CHANGES_REQUESTED")
        ))
        .unwrap();

        assert_eq!(view.pr.review_state, Some(ReviewState::Approved));
    }

    #[test]
    fn test_a_changes_requested_decision_wins_over_an_approval() {
        let view = pr_view(&format!(
            r#""isDraft":false,"reviewDecision":"CHANGES_REQUESTED","latestReviews":[{}]"#,
            review("APPROVED")
        ))
        .unwrap();

        assert_eq!(view.pr.review_state, Some(ReviewState::ChangesRequested));
    }

    #[test]
    fn test_a_required_review_not_yet_given_sets_no_state() {
        let view = pr_view(&format!(
            r#""isDraft":false,"reviewDecision":"REVIEW_REQUIRED","latestReviews":[{}]"#,
            review("APPROVED")
        ))
        .unwrap();

        assert_eq!(view.pr.review_state, None);
    }

    #[test]
    fn test_a_merged_pr_has_no_review_state() {
        let json = format!(
            r#"{{"number":50,"url":"https://github.com/org/repo/pull/50","state":"MERGED","isDraft":false,"reviewDecision":"","latestReviews":[{}]}}"#,
            review("APPROVED")
        );
        let view = parse_pr_view(json.as_bytes()).unwrap();

        assert!(!view.open);
        assert_eq!(view.pr.review_state, None);
    }

    #[test]
    fn test_comments_and_dismissed_reviews_set_no_state() {
        let view = pr_view(&format!(
            r#""isDraft":false,"reviewDecision":"","latestReviews":[{},{}]"#,
            review("COMMENTED"),
            review("DISMISSED")
        ))
        .unwrap();

        assert_eq!(view.pr.review_state, None);
    }

    #[test]
    fn test_a_cache_entry_from_before_review_support_still_parses() {
        let temp_file = std::env::temp_dir().join(format!(
            "test_cache_old_{}_{}.json",
            std::process::id(),
            current_timestamp()
        ));
        fs::write(
            &temp_file,
            r#"{"timestamp":1000,"pr":{"number":50,"url":"https://github.com/org/repo/pull/50"}}"#,
        )
        .unwrap();

        let loaded = read_cache_entry(&temp_file).unwrap();
        let _ = fs::remove_file(&temp_file);

        assert_eq!(loaded.pr.unwrap().review_state, None);
    }

    fn review_threads(resolved: &[bool], has_next_page: bool) -> String {
        let nodes: Vec<String> = resolved
            .iter()
            .map(|r| format!(r#"{{"isResolved":{r},"isOutdated":true}}"#))
            .collect();
        format!(
            r#"{{"data":{{"repository":{{"pullRequest":{{"reviewThreads":{{"pageInfo":{{"hasNextPage":{has_next_page}}},"nodes":[{}]}}}}}}}}}}"#,
            nodes.join(",")
        )
    }

    #[test]
    fn test_counts_unresolved_threads_outdated_ones_included() {
        let json = review_threads(&[false, true, false, false], false);

        assert_eq!(
            parse_unresolved_threads(json.as_bytes()),
            Some(UnresolvedThreads {
                count: 3,
                more: false
            })
        );
    }

    #[test]
    fn test_threads_past_the_first_page_mark_the_count_as_a_lower_bound() {
        let json = review_threads(&[false, false], true);

        assert_eq!(
            parse_unresolved_threads(json.as_bytes()),
            Some(UnresolvedThreads {
                count: 2,
                more: true
            })
        );
    }

    #[test]
    fn test_a_graphql_error_gives_no_thread_count() {
        let json = r#"{"data":{"repository":null},"errors":[{"message":"Could not resolve"}]}"#;

        assert_eq!(parse_unresolved_threads(json.as_bytes()), None);
    }

    #[test]
    fn test_the_pr_url_names_the_repo_and_number_to_query() {
        assert_eq!(
            pr_coordinates("https://github.com/xpepper/copilot-powerline/pull/50"),
            Some(("xpepper", "copilot-powerline", 50))
        );
    }

    #[test]
    fn test_an_unexpected_pr_url_names_nothing_to_query() {
        for url in [
            "https://evil.example.com/o/r/pull/50",
            "https://github.com/o/r/issues/50",
            "https://github.com/o/r/pull/x",
            "https://github.com/o/pull/50",
            "https://github.com//r/pull/50",
            "https://github.com/o//pull/50",
        ] {
            assert_eq!(pr_coordinates(url), None, "{url}");
        }
    }

    #[test]
    fn test_get_pr_info_when_gh_not_available() {
        let res = get_pr_info(Path::new("/some/repo"), "main", &PrConfig::default(), false);
        assert!(res.is_none());
    }
}
