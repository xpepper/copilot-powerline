//! Runs the built binary against a throwaway `HOME` to check that
//! `COPILOT_POWERLINE_LOG` makes each refresh append one log line. The unit
//! tests in `src/payload_log.rs` cover the line format; this covers `main`.

mod common;

use common::{Sandbox, payload};
use std::fs;

const LOG_ENV: &str = "COPILOT_POWERLINE_LOG";

#[test]
fn test_refresh_appends_one_line_per_refresh_when_the_log_is_enabled() {
    let sandbox = Sandbox::new();
    let log = sandbox.home().join("payloads.log");
    let input = payload("wiring", 5_000_000_000);

    sandbox.refresh_with_env(&input, &[], &[(LOG_ENV, &log)]);
    sandbox.refresh_with_env(&input, &[], &[(LOG_ENV, &log)]);

    let content = fs::read_to_string(&log).unwrap();
    let lines: Vec<_> = content.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("total_tokens=1100 nano_aiu=5000000000"));
    assert!(!content.contains("wiring"));
}

#[test]
fn test_refresh_writes_no_log_by_default() {
    let sandbox = Sandbox::new();

    sandbox.refresh(&payload("wiring", 0), &[]);

    assert!(!sandbox.home().join("payloads.log").exists());
}
