use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn default_db_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".copilot").join("session-store.db"))
}

pub fn get_month_other_sessions_nano(db_path: &Path, session_id: Option<&str>) -> u64 {
    if !db_path.exists() {
        return 0;
    }

    let conn_result = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    );

    let conn = match conn_result {
        Ok(c) => c,
        Err(_) => return 0,
    };

    let _ = conn.busy_timeout(Duration::from_millis(500));

    let sess_id = session_id.unwrap_or("");
    let query = "
        SELECT COALESCE(SUM(total_nano_aiu), 0)
        FROM assistant_usage_events
        WHERE session_id != ?1 AND created_at >= strftime('%Y-%m-01', 'now')
    ";

    let mut stmt = match conn.prepare(query) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    stmt.query_row([sess_id], |row| row.get::<_, i64>(0))
        .map(|val| val.max(0) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_existent_db_returns_zero() {
        let path = Path::new("/non/existent/path/db.sqlite");
        assert_eq!(get_month_other_sessions_nano(path, None), 0);
    }

    #[test]
    fn test_query_filters_session_and_date() {
        // Create an actual sqlite db file for testing
        let temp_dir = std::env::temp_dir();
        let db_file = temp_dir.join(format!("test_copilot_{}.db", std::process::id()));

        let conn = Connection::open(&db_file).unwrap();
        conn.execute(
            "CREATE TABLE assistant_usage_events (
                session_id TEXT,
                total_nano_aiu INTEGER,
                created_at TEXT
            )",
            [],
        )
        .unwrap();

        // 1. Current session in current month -> should be ignored (because session_id == current)
        conn.execute(
            "INSERT INTO assistant_usage_events (session_id, total_nano_aiu, created_at)
             VALUES (?1, ?2, datetime('now'))",
            ("curr-session", 100_000_000_000i64),
        )
        .unwrap();

        // 2. Another session in current month -> should be included (300 billion)
        conn.execute(
            "INSERT INTO assistant_usage_events (session_id, total_nano_aiu, created_at)
             VALUES (?1, ?2, datetime('now'))",
            ("other-session-1", 300_000_000_000i64),
        )
        .unwrap();

        // 3. Another session in last month -> should be ignored (date < start of month)
        conn.execute(
            "INSERT INTO assistant_usage_events (session_id, total_nano_aiu, created_at)
             VALUES (?1, ?2, datetime('now', '-2 months'))",
            ("other-session-old", 500_000_000_000i64),
        )
        .unwrap();

        let total = get_month_other_sessions_nano(&db_file, Some("curr-session"));
        assert_eq!(total, 300_000_000_000);

        let _ = std::fs::remove_file(&db_file);
    }
}
