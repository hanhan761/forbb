//! Small, provider-local cache for web-grounded answers.
//!
//! Web lookup is intentionally cached only in memory.  This keeps the hot
//! path light, avoids persisting third-party content without an explicit
//! export, and is enough to reuse the professor/lab/paper lookups made during
//! one app session or interview preparation window.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::provider::StreamSource;

const DEFAULT_MAX_ENTRIES: usize = 64;
const DEFAULT_TTL: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Debug, Clone)]
struct CachedWebAnswer {
    answer: String,
    sources: Vec<StreamSource>,
    stored_at: Instant,
    last_accessed: Instant,
}

/// Bounded TTL cache for complete web-grounded answers.
#[derive(Debug)]
pub struct WebAnswerCache {
    entries: HashMap<String, CachedWebAnswer>,
    max_entries: usize,
    ttl: Duration,
}

impl Default for WebAnswerCache {
    fn default() -> Self {
        Self::with_limits(DEFAULT_MAX_ENTRIES, DEFAULT_TTL)
    }
}

impl WebAnswerCache {
    pub fn with_limits(max_entries: usize, ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries: max_entries.max(1),
            ttl,
        }
    }

    /// Normalize a query so whitespace and casing do not create duplicate
    /// entries.  The caller should include model/reasoning in the outer key.
    pub fn normalize_key(value: &str) -> String {
        value
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase()
    }

    pub fn get(&mut self, key: &str) -> Option<(String, Vec<StreamSource>)> {
        let normalized = Self::normalize_key(key);
        if normalized.is_empty() {
            return None;
        }

        let now = Instant::now();
        let expired = self
            .entries
            .get(&normalized)
            .map(|entry| now.duration_since(entry.stored_at) > self.ttl)
            .unwrap_or(false);

        if expired {
            self.entries.remove(&normalized);
            return None;
        }

        self.entries.get_mut(&normalized).map(|entry| {
            entry.last_accessed = now;
            (entry.answer.clone(), entry.sources.clone())
        })
    }

    pub fn insert(&mut self, key: &str, answer: String, sources: Vec<StreamSource>) {
        let normalized = Self::normalize_key(key);
        if normalized.is_empty() {
            return;
        }

        let now = Instant::now();
        if self.entries.len() >= self.max_entries && !self.entries.contains_key(&normalized) {
            if let Some(oldest_key) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_accessed)
                .map(|(key, _)| key.clone())
            {
                self.entries.remove(&oldest_key);
            }
        }

        self.entries.insert(
            normalized,
            CachedWebAnswer {
                answer,
                sources,
                stored_at: now,
                last_accessed: now,
            },
        );
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::WebAnswerCache;
    use std::time::Duration;

    #[test]
    fn normalizes_query_keys_and_returns_sources() {
        let mut cache = WebAnswerCache::with_limits(4, Duration::from_secs(60));
        cache.insert(
            "  Latest   paper   on   robotics ",
            "answer".to_string(),
            vec![],
        );

        let hit = cache.get("latest paper on robotics");
        assert_eq!(hit.map(|(answer, _)| answer), Some("answer".to_string()));
    }

    #[test]
    fn evicts_oldest_entry_when_capacity_is_reached() {
        let mut cache = WebAnswerCache::with_limits(1, Duration::from_secs(60));
        cache.insert("first", "one".to_string(), vec![]);
        cache.insert("second", "two".to_string(), vec![]);

        assert!(cache.get("first").is_none());
        assert_eq!(cache.get("second").map(|(answer, _)| answer), Some("two".to_string()));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn expires_entries_without_sleeping() {
        let mut cache = WebAnswerCache::with_limits(1, Duration::ZERO);
        cache.insert("stale", "answer".to_string(), vec![]);
        assert!(cache.get("stale").is_none());
    }
}
