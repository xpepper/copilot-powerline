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

/// Test helper: owned segment names, the shape `segments` comes in.
#[cfg(test)]
fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(ToString::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_default_segments_use_segment_names() {
        // `config` must not depend on a segment, so its defaults spell the
        // names out; this catches a renamed segment they still list.
        let known = [
            tokens::NAME,
            session_cost::NAME,
            month_cost::NAME,
            cycle_cost::NAME,
            cache::NAME,
            reasoning::NAME,
            total_tokens::NAME,
            cache_expiry::NAME,
            model::NAME,
            pr::NAME,
        ];
        let config = crate::config::Config::default();
        for name in config.segments.iter().chain(&config.compact_segments) {
            assert!(known.contains(&name.as_str()), "unknown segment {name}");
        }
    }
}
