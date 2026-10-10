pub mod cache;
pub mod cache_expiry;
pub mod cycle_cost;
pub mod model;
pub mod month_cost;
pub mod pr;
pub mod reasoning;
pub mod session_cost;
pub mod spend_format;
pub mod spend_total;
pub mod tokens;
pub mod total_tokens;

/// Whether `name` is one of the `segments` active for the current display
/// mode. Callers combine it with the segment's own flags to decide whether a
/// lookup that touches the disk, the database or the network is worth running.
pub fn is_listed(segments: &[String], name: &str) -> bool {
    segments.iter().any(|s| s == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn test_listed_segment_is_found() {
        assert!(is_listed(&names(&["tokens", "pr"]), "pr"));
    }

    #[test]
    fn test_unlisted_segment_is_not_found() {
        assert!(!is_listed(&names(&["tokens"]), "pr"));
        assert!(!is_listed(&[], "pr"));
    }

    #[test]
    fn test_match_is_exact() {
        assert!(!is_listed(&names(&["cache_expiry"]), "cache"));
        assert!(!is_listed(&names(&["cache"]), "cache_expiry"));
    }
}
