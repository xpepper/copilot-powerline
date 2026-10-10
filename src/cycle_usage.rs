//! GitHub's count of AI credits used in the current billing cycle, across
//! every Copilot surface (not just Copilot CLI).
//!
//! It comes from the internal `/copilot_internal/user` API, which takes about
//! a second, so it follows the `pr` segment's pattern: the status line only
//! reads a disk cache, and a stale cache triggers a throttled, detached
//! refresh (this binary re-invoked with `--fetch-cycle-usage`).

use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CycleUsage {
    /// AI credits used so far this cycle (1 credit = USD 0.01).
    pub credits: f64,
    /// When the cycle ends and the counter resets, in Unix seconds (UTC).
    pub resets_at: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CacheEntry {
    timestamp: u64,
    /// `None` when the last refresh failed, which hides the segment.
    usage: Option<CycleUsage>,
}

/// Minimum time between two background refreshes, even while failing.
const THROTTLE_SECONDS: u64 = 15;

pub fn cache_path() -> PathBuf {
    crate::state::base_dir().join("cycle_usage.json")
}

/// The current cycle's usage, if it is cached.
///
/// Never blocks on the network: a stale or missing cache calls
/// `spawn_refresh` (at most once per throttle window) and returns whatever
/// was cached. Usage cached for a cycle that has since ended is not returned.
pub fn get_cycle_usage(
    cache_path: &Path,
    ttl_seconds: u64,
    now: u64,
    spawn_refresh: impl FnOnce(&Path),
) -> Option<CycleUsage> {
    let entry = read_entry(cache_path);

    let fresh = entry
        .as_ref()
        .is_some_and(|e| now.saturating_sub(e.timestamp) < ttl_seconds);
    if !fresh && crate::github::try_claim_spawn(cache_path, now, THROTTLE_SECONDS) {
        spawn_refresh(cache_path);
    }

    entry
        .and_then(|e| e.usage)
        .filter(|usage| now < usage.resets_at)
}

fn read_entry(cache_path: &Path) -> Option<CacheEntry> {
    std::fs::read_to_string(cache_path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
}

/// Re-invokes this binary as a detached `--fetch-cycle-usage` worker.
pub fn spawn_background_fetch(cache_path: &Path) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe)
            .arg("--fetch-cycle-usage")
            .arg(cache_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

/// Calls the API and writes the result to the cache.
///
/// This blocks on a network call, so it must only ever run in the detached
/// worker started by `spawn_background_fetch`, never on the hot path.
pub fn fetch_and_write_cycle_cache(cache_path: &Path) {
    let now = crate::github::current_timestamp();
    let previous = read_entry(cache_path);
    let mut command = api_command(std::env::var_os("COPILOT_GITHUB_TOKEN"));
    let entry = match crate::github::output_with_timeout(&mut command, crate::github::GH_TIMEOUT) {
        Some(out) => entry_from(out.status.success(), &out.stdout, now, previous.as_ref()),
        None => entry_from(false, &[], now, previous.as_ref()),
    };
    if let Ok(json) = serde_json::to_string(&entry) {
        let _ = crate::state::write_private_atomic(cache_path, json.as_bytes());
    }
    let _ = std::fs::remove_file(crate::github::lock_file_path(cache_path));
}

/// `gh api` for the Copilot user endpoint. gh picks its token from
/// `GH_TOKEN`, `GITHUB_TOKEN`, then its own login; Copilot CLI checks
/// `COPILOT_GITHUB_TOKEN` before those, so it is passed on as `GH_TOKEN`.
fn api_command(copilot_token: Option<OsString>) -> Command {
    let mut command = Command::new("gh");
    command.args(["api", "/copilot_internal/user"]);
    if let Some(token) = copilot_token.filter(|t| !t.is_empty()) {
        command.env("GH_TOKEN", token);
    }
    command
}

/// The cache entry a refresh produces. A failed refresh keeps the last known
/// usage (and backs off for a full TTL) instead of hiding the segment on a
/// transient error; usage from an ended cycle is dropped on read anyway.
fn entry_from(
    succeeded: bool,
    stdout: &[u8],
    now: u64,
    previous: Option<&CacheEntry>,
) -> CacheEntry {
    let usage = succeeded
        .then(|| std::str::from_utf8(stdout).ok())
        .flatten()
        .and_then(parse_user_response)
        .or_else(|| previous.and_then(|p| p.usage.clone()));
    CacheEntry {
        timestamp: now,
        usage,
    }
}

#[derive(Deserialize)]
struct UserResponse {
    quota_reset_date: Option<String>,
    quota_snapshots: Option<QuotaSnapshots>,
}

#[derive(Deserialize)]
struct QuotaSnapshots {
    premium_interactions: Option<PremiumSnapshot>,
}

#[derive(Deserialize)]
struct PremiumSnapshot {
    unlimited: Option<bool>,
    credits_used: Option<f64>,
    entitlement: Option<f64>,
    quota_remaining: Option<f64>,
}

impl PremiumSnapshot {
    /// With no per-user budget (`unlimited`), `credits_used` is the cycle
    /// total; with one, the used part of the budget is `entitlement -
    /// quota_remaining`, as VS Code computes it.
    fn credits(&self) -> Option<f64> {
        match self.unlimited? {
            true => self.credits_used,
            false => {
                let entitlement = self.entitlement.filter(|e| *e > 0.0)?;
                Some((entitlement - self.quota_remaining?).max(0.0))
            }
        }
    }
}

/// Reads the cycle usage from a `/copilot_internal/user` response, or `None`
/// when a field it needs is missing (never a guessed 0).
pub fn parse_user_response(json: &str) -> Option<CycleUsage> {
    let response: UserResponse = serde_json::from_str(json).ok()?;
    Some(CycleUsage {
        credits: response.quota_snapshots?.premium_interactions?.credits()?,
        resets_at: utc_midnight(&response.quota_reset_date?)?,
    })
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Midnight UTC of a `YYYY-MM-DD` date, in Unix seconds.
fn utc_midnight(date: &str) -> Option<u64> {
    let mut parts = date.split('-');
    let (year, month, day) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return None;
    }
    let (year, month, day): (i64, i64, i64) =
        (year.parse().ok()?, month.parse().ok()?, day.parse().ok()?);
    if !(1..=12).contains(&month) || !(1..=days_in_month(year, month)).contains(&day) {
        return None;
    }

    // Days since 1970-01-01, from Howard Hinnant's `days_from_civil`.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;

    u64::try_from(days * 86_400).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNLIMITED: &str = include_str!("../tests/fixtures/copilot-user-unlimited.json");
    const BUDGET: &str = include_str!("../tests/fixtures/copilot-user-budget.json");

    fn edited(json: &str, edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut value: serde_json::Value = serde_json::from_str(json).unwrap();
        edit(&mut value);
        value.to_string()
    }

    fn premium(value: &mut serde_json::Value) -> &mut serde_json::Value {
        &mut value["quota_snapshots"]["premium_interactions"]
    }

    #[test]
    fn test_unlimited_plan_reads_credits_used() {
        assert_eq!(
            parse_user_response(UNLIMITED),
            Some(CycleUsage {
                credits: 15649.0,
                resets_at: 1_793_491_200, // 2026-11-01T00:00:00Z
            })
        );
    }

    #[test]
    fn test_per_user_budget_counts_entitlement_minus_remaining() {
        // The fixture's credits_used (99999) must be ignored.
        let usage = parse_user_response(BUDGET).unwrap();
        assert_eq!(usage.credits, 2000.0);
    }

    #[test]
    fn test_per_user_budget_never_goes_negative() {
        let json = edited(BUDGET, |v| premium(v)["quota_remaining"] = 3500.into());
        assert_eq!(parse_user_response(&json).unwrap().credits, 0.0);
    }

    #[test]
    fn test_missing_fields_give_no_usage() {
        type Edit = fn(&mut serde_json::Value);
        let cases: [(&str, Edit); 6] = [
            ("credits_used", |v| {
                premium(v).as_object_mut().unwrap().remove("credits_used");
            }),
            ("unlimited", |v| {
                premium(v).as_object_mut().unwrap().remove("unlimited");
            }),
            ("premium_interactions", |v| {
                v["quota_snapshots"]
                    .as_object_mut()
                    .unwrap()
                    .remove("premium_interactions");
            }),
            ("quota_reset_date", |v| {
                v.as_object_mut().unwrap().remove("quota_reset_date");
            }),
            ("malformed quota_reset_date", |v| {
                v["quota_reset_date"] = "next month".into();
            }),
            ("budget without quota_remaining", |v| {
                *premium(v) = serde_json::json!({"unlimited": false, "entitlement": 3000});
            }),
        ];
        for (name, edit) in cases {
            let json = edited(UNLIMITED, edit);
            assert_eq!(parse_user_response(&json), None, "{name}");
        }
        assert_eq!(parse_user_response("not json"), None);
    }

    const NOW: u64 = 1_790_000_000; // 2026-09-21, inside the cycle ending 2026-11-01
    const TTL: u64 = 300;

    fn usage() -> CycleUsage {
        CycleUsage {
            credits: 15649.0,
            resets_at: 1_793_491_200,
        }
    }

    fn cache_with(label: &str, entry: Option<CacheEntry>) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "copilot-powerline-cycle-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cycle_usage.json");
        if let Some(entry) = entry {
            std::fs::write(&path, serde_json::to_string(&entry).unwrap()).unwrap();
        }
        path
    }

    /// Calls `get_cycle_usage` and reports whether it asked for a refresh.
    fn get(path: &Path, now: u64) -> (Option<CycleUsage>, bool) {
        let mut spawned = false;
        let usage = get_cycle_usage(path, TTL, now, |_| spawned = true);
        (usage, spawned)
    }

    #[test]
    fn test_fresh_cache_is_used_without_refreshing() {
        let path = cache_with(
            "fresh",
            Some(CacheEntry {
                timestamp: NOW - 10,
                usage: Some(usage()),
            }),
        );
        assert_eq!(get(&path, NOW), (Some(usage()), false));
    }

    #[test]
    fn test_stale_cache_is_shown_while_one_refresh_starts() {
        let path = cache_with(
            "stale",
            Some(CacheEntry {
                timestamp: NOW - TTL,
                usage: Some(usage()),
            }),
        );
        assert_eq!(get(&path, NOW), (Some(usage()), true));
        // Throttled: the next refresh within the window starts nothing.
        assert_eq!(get(&path, NOW + 1), (Some(usage()), false));
    }

    #[test]
    fn test_missing_cache_starts_a_refresh() {
        let path = cache_with("missing", None);
        assert_eq!(get(&path, NOW), (None, true));
    }

    #[test]
    fn test_usage_from_an_ended_cycle_is_hidden() {
        let path = cache_with(
            "ended",
            Some(CacheEntry {
                timestamp: NOW,
                usage: Some(usage()),
            }),
        );
        let reset = usage().resets_at;
        assert_eq!(get(&path, reset - 1).0, Some(usage()));
        assert_eq!(get(&path, reset).0, None);
    }

    #[test]
    fn test_api_command_calls_the_copilot_user_endpoint() {
        let command = api_command(None);
        assert_eq!(command.get_program(), "gh");
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args, ["api", "/copilot_internal/user"]);
        assert_eq!(command.get_envs().count(), 0);
    }

    #[test]
    fn test_api_command_passes_copilot_token_to_gh() {
        let command = api_command(Some("copilot-token".into()));
        let envs: Vec<_> = command.get_envs().collect();
        assert_eq!(
            envs,
            [("GH_TOKEN".as_ref(), Some("copilot-token".as_ref()))]
        );
    }

    #[test]
    fn test_refresh_result_becomes_the_cache_entry() {
        let ok = entry_from(true, UNLIMITED.as_bytes(), NOW, None);
        assert_eq!(ok.timestamp, NOW);
        assert_eq!(ok.usage, Some(usage()));
        // A failed call or an unexpected response hides the segment.
        assert_eq!(
            entry_from(false, UNLIMITED.as_bytes(), NOW, None).usage,
            None
        );
        assert_eq!(entry_from(true, b"{}", NOW, None).usage, None);
    }

    #[test]
    fn test_failed_refresh_keeps_the_last_known_usage() {
        let previous = Some(CacheEntry {
            timestamp: NOW - TTL,
            usage: Some(usage()),
        });
        let failed = entry_from(false, &[], NOW, previous.as_ref());
        assert_eq!(failed.timestamp, NOW);
        assert_eq!(failed.usage, Some(usage()));
        // An unexpected response is a failure too.
        assert_eq!(
            entry_from(true, b"{}", NOW, previous.as_ref()).usage,
            Some(usage())
        );
    }

    #[test]
    fn test_successful_refresh_replaces_the_last_known_usage() {
        let previous = CacheEntry {
            timestamp: NOW - TTL,
            usage: Some(CycleUsage {
                credits: 1.0,
                resets_at: 1_793_491_200,
            }),
        };
        let ok = entry_from(true, UNLIMITED.as_bytes(), NOW, Some(&previous));
        assert_eq!(ok.usage, Some(usage()));
    }

    #[test]
    fn test_utc_midnight_rejects_days_the_month_does_not_have() {
        assert_eq!(utc_midnight("2026-02-31"), None);
        assert_eq!(utc_midnight("2026-02-29"), None);
        assert_eq!(utc_midnight("2026-04-31"), None);
        assert_eq!(utc_midnight("2026-00-10"), None);
        assert_eq!(utc_midnight("2026-01-00"), None);
        assert!(utc_midnight("2028-02-29").is_some());
        assert!(utc_midnight("2026-12-31").is_some());
    }

    #[test]
    fn test_utc_midnight() {
        assert_eq!(utc_midnight("1970-01-01"), Some(0));
        assert_eq!(utc_midnight("2000-03-01"), Some(951_868_800));
        assert_eq!(utc_midnight("2028-02-29"), Some(1_835_395_200));
        assert_eq!(utc_midnight("2026-13-01"), None);
        assert_eq!(utc_midnight("2026-11-1"), None);
        assert_eq!(utc_midnight("1969-12-31"), None);
    }
}
