//! Runs the built binary against a config file and a Copilot database that
//! live in a throwaway `HOME`, and checks the rendered `month_cost` segment.
//! The unit tests cover the query and the config parsing from string
//! literals; this covers the whole path from files on disk to the screen, so
//! a dependency bump that breaks it (`rusqlite`, `toml`, `dirs`) fails here.
//! `db.rs` and `Config::load_from_file_or_default` turn errors into `0` and
//! defaults, so without this a broken upgrade would degrade silently.
#![cfg(unix)]

mod common;

use common::{LONG_AGO, NOW, SESSION, Sandbox, assert_shows, create_db, insert_usage};

/// $250.00 in nano AIU.
const OTHER_SPEND: i64 = 25_000_000_000_000;

/// Values the built-in defaults never produce (a custom prefix, AIC shown), so
/// the segment only comes out right when the file is read.
const CONFIG: &str = r#"
theme = "plain"
segments = ["month_cost"]

[month_cost]
prefix = "MTD:"
show_aic = true
"#;

fn seed(conn: &rusqlite::Connection) {
    insert_usage(conn, "other", OTHER_SPEND, NOW);
    // Excluded: the current session comes live from the payload, and a past
    // month is out of the window.
    insert_usage(conn, SESSION, 1_000_000_000_000, NOW);
    insert_usage(conn, "other", 9_000_000_000_000, LONG_AGO);
}

#[test]
fn test_month_cost_adds_the_default_database_to_the_session_spend() {
    let sandbox = Sandbox::new();
    sandbox.write_config(CONFIG);
    seed(&sandbox.create_default_db());

    assert_shows(&sandbox.refresh_plain(), "MTD: $300.00 (30000 AIC)");
}

#[test]
fn test_month_cost_reads_the_database_named_by_db_path() {
    let sandbox = Sandbox::new();
    let db_path = sandbox.home().join("elsewhere").join("usage.db");
    sandbox.write_config(&format!("{CONFIG}db_path = \"{}\"\n", db_path.display()));
    seed(&create_db(&db_path));

    assert_shows(&sandbox.refresh_plain(), "MTD: $300.00 (30000 AIC)");
}
