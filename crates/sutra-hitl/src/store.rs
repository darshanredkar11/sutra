use std::collections::HashMap;

use sutra_common::error::{SutraError, SutraResult};
use sutra_schema::v1::Engine;

use crate::types::{FeedbackEntry, FeedbackMetrics, FeedbackVerdict};

pub trait FeedbackStore: Send + Sync {
    fn store(&mut self, entry: FeedbackEntry) -> SutraResult<()>;
    fn get_by_finding_id(&self, finding_id: &str) -> SutraResult<Vec<FeedbackEntry>>;
    fn get_by_engine(&self, engine: &Engine) -> SutraResult<Vec<FeedbackEntry>>;
    fn get_all(&self) -> SutraResult<Vec<FeedbackEntry>>;
    fn metrics(&self) -> SutraResult<FeedbackMetrics>;
    fn clear(&mut self) -> SutraResult<()>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct InMemoryFeedbackStore {
    entries: Vec<FeedbackEntry>,
}

impl InMemoryFeedbackStore {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            entries: Vec::with_capacity(cap),
        }
    }
}

impl FeedbackStore for InMemoryFeedbackStore {
    fn store(&mut self, entry: FeedbackEntry) -> SutraResult<()> {
        if entry.id.is_empty() {
            return Err(SutraError::config("feedback entry id cannot be empty"));
        }
        if self.entries.iter().any(|e| e.id == entry.id) {
            return Err(SutraError::config(format!(
                "duplicate feedback entry id: {}",
                entry.id
            )));
        }
        self.entries.push(entry);
        Ok(())
    }

    fn get_by_finding_id(&self, finding_id: &str) -> SutraResult<Vec<FeedbackEntry>> {
        Ok(self
            .entries
            .iter()
            .filter(|e| e.finding_id == finding_id)
            .cloned()
            .collect())
    }

    fn get_by_engine(&self, engine: &Engine) -> SutraResult<Vec<FeedbackEntry>> {
        Ok(self
            .entries
            .iter()
            .filter(|e| e.engine == *engine)
            .cloned()
            .collect())
    }

    fn get_all(&self) -> SutraResult<Vec<FeedbackEntry>> {
        Ok(self.entries.clone())
    }

    fn metrics(&self) -> SutraResult<FeedbackMetrics> {
        let mut m = FeedbackMetrics::new();
        m.total_entries = self.entries.len();

        let mut correct: usize = 0;
        let mut incorrect: usize = 0;
        let mut uncertain: usize = 0;
        let mut engine_counts: HashMap<String, (usize, usize)> = HashMap::new(); // (correct, total_valid)
        let mut unique_findings: std::collections::HashSet<&str> = std::collections::HashSet::new();

        for entry in &self.entries {
            unique_findings.insert(&entry.finding_id);
            match entry.verdict {
                FeedbackVerdict::Correct => {
                    correct += 1;
                    let e = engine_counts.entry(entry.engine.to_string()).or_insert((0, 0));
                    e.0 += 1;
                    e.1 += 1;
                }
                FeedbackVerdict::Incorrect => {
                    incorrect += 1;
                    let e = engine_counts.entry(entry.engine.to_string()).or_insert((0, 0));
                    e.1 += 1;
                }
                FeedbackVerdict::Uncertain => {
                    uncertain += 1;
                }
            }
        }

        m.correct_count = correct;
        m.incorrect_count = incorrect;
        m.uncertain_count = uncertain;
        m.total_findings_with_feedback = unique_findings.len();

        for (engine_name, (correct_count, total_valid)) in &engine_counts {
            if *total_valid > 0 {
                m.precision_by_engine
                    .insert(engine_name.clone(), *correct_count as f64 / *total_valid as f64);
            }
        }

        Ok(m)
    }

    fn clear(&mut self) -> SutraResult<()> {
        self.entries.clear();
        Ok(())
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FeedbackEntry;
    use sutra_schema::v1::Engine;

    fn make_entry(id: &str, finding_id: &str, engine: Engine, verdict: FeedbackVerdict) -> FeedbackEntry {
        FeedbackEntry::new(id, finding_id, engine, "f.rs", 1, verdict, "tester")
    }

    #[test]
    fn test_store_and_get_all() {
        let mut store = InMemoryFeedbackStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);

        store.store(make_entry("e1", "f1", Engine::Mgtg, FeedbackVerdict::Correct)).unwrap();
        store.store(make_entry("e2", "f2", Engine::Dependency, FeedbackVerdict::Incorrect)).unwrap();

        assert_eq!(store.len(), 2);
        assert!(!store.is_empty());

        let all = store.get_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_store_duplicate_id_fails() {
        let mut store = InMemoryFeedbackStore::new();
        store.store(make_entry("e1", "f1", Engine::Mgtg, FeedbackVerdict::Correct)).unwrap();
        let err = store.store(make_entry("e1", "f2", Engine::Process, FeedbackVerdict::Correct)).unwrap_err();
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn test_store_empty_id_fails() {
        let mut store = InMemoryFeedbackStore::new();
        let err = store.store(make_entry("", "f1", Engine::Mgtg, FeedbackVerdict::Correct)).unwrap_err();
        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_get_by_finding_id() {
        let mut store = InMemoryFeedbackStore::new();
        store.store(make_entry("e1", "f1", Engine::Mgtg, FeedbackVerdict::Correct)).unwrap();
        store.store(make_entry("e2", "f1", Engine::Dependency, FeedbackVerdict::Incorrect)).unwrap();
        store.store(make_entry("e3", "f2", Engine::Process, FeedbackVerdict::Correct)).unwrap();

        let for_f1 = store.get_by_finding_id("f1").unwrap();
        assert_eq!(for_f1.len(), 2);
        assert!(for_f1.iter().all(|e| e.finding_id == "f1"));

        let for_f2 = store.get_by_finding_id("f2").unwrap();
        assert_eq!(for_f2.len(), 1);
    }

    #[test]
    fn test_get_by_finding_id_missing() {
        let store = InMemoryFeedbackStore::new();
        let result = store.get_by_finding_id("nonexistent").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_by_engine() {
        let mut store = InMemoryFeedbackStore::new();
        store.store(make_entry("e1", "f1", Engine::Mgtg, FeedbackVerdict::Correct)).unwrap();
        store.store(make_entry("e2", "f2", Engine::Mgtg, FeedbackVerdict::Incorrect)).unwrap();
        store.store(make_entry("e3", "f3", Engine::Process, FeedbackVerdict::Correct)).unwrap();

        let mgtg = store.get_by_engine(&Engine::Mgtg).unwrap();
        assert_eq!(mgtg.len(), 2);

        let process = store.get_by_engine(&Engine::Process).unwrap();
        assert_eq!(process.len(), 1);

        let ml = store.get_by_engine(&Engine::Ml).unwrap();
        assert!(ml.is_empty());
    }

    #[test]
    fn test_metrics_empty() {
        let store = InMemoryFeedbackStore::new();
        let m = store.metrics().unwrap();
        assert_eq!(m.total_entries, 0);
        assert_eq!(m.total_findings_with_feedback, 0);
    }

    #[test]
    fn test_metrics_with_entries() {
        let mut store = InMemoryFeedbackStore::new();
        store.store(make_entry("e1", "f1", Engine::Mgtg, FeedbackVerdict::Correct)).unwrap();
        store.store(make_entry("e2", "f1", Engine::Mgtg, FeedbackVerdict::Correct)).unwrap();
        store.store(make_entry("e3", "f2", Engine::Mgtg, FeedbackVerdict::Incorrect)).unwrap();
        store.store(make_entry("e4", "f3", Engine::Process, FeedbackVerdict::Correct)).unwrap();

        let m = store.metrics().unwrap();
        assert_eq!(m.total_entries, 4);
        assert_eq!(m.correct_count, 3);
        assert_eq!(m.incorrect_count, 1);
        assert_eq!(m.uncertain_count, 0);
        assert_eq!(m.total_findings_with_feedback, 3);

        let mgtg_precision = m.precision_by_engine.get("mgtg").unwrap();
        assert!((*mgtg_precision - 2.0 / 3.0).abs() < 1e-9);

        let process_precision = m.precision_by_engine.get("process").unwrap();
        assert!((*process_precision - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_clear() {
        let mut store = InMemoryFeedbackStore::new();
        store.store(make_entry("e1", "f1", Engine::Mgtg, FeedbackVerdict::Correct)).unwrap();
        assert_eq!(store.len(), 1);
        store.clear().unwrap();
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_with_capacity() {
        let store: InMemoryFeedbackStore = InMemoryFeedbackStore::with_capacity(100);
        assert!(store.is_empty());
    }

    #[test]
    fn test_store_many_entries() {
        let mut store = InMemoryFeedbackStore::new();
        for i in 0..100 {
            let entry = make_entry(&format!("e{}", i), &format!("f{}", i % 10), Engine::Mgtg, FeedbackVerdict::Correct);
            store.store(entry).unwrap();
        }
        assert_eq!(store.len(), 100);
    }

    // ── Stress tests ──────────────────────────────────────────────────

    #[test]
    fn test_stress_store_10000_entries() {
        let mut store = InMemoryFeedbackStore::new();
        for i in 0..10_000 {
            let entry = make_entry(&format!("e{}", i), "f1", Engine::Mgtg, FeedbackVerdict::Correct);
            store.store(entry).unwrap();
        }
        assert_eq!(store.len(), 10_000);
    }

    #[test]
    fn test_stress_store_1000_unique_findings() {
        let mut store = InMemoryFeedbackStore::new();
        for i in 0..1_000 {
            for j in 0..5 {
                let entry = make_entry(
                    &format!("e{}_{}", i, j),
                    &format!("f{}", i),
                    Engine::Mgtg,
                    if j % 2 == 0 { FeedbackVerdict::Correct } else { FeedbackVerdict::Incorrect },
                );
                store.store(entry).unwrap();
            }
        }
        assert_eq!(store.len(), 5_000);
        let metrics = store.metrics().unwrap();
        assert_eq!(metrics.total_findings_with_feedback, 1_000);
    }

    #[test]
    fn test_stress_get_by_finding_id_across_10000_entries() {
        let mut store = InMemoryFeedbackStore::new();
        for i in 0..10_000 {
            let finding_id = format!("f{}", i % 100);
            let entry = make_entry(&format!("e{}", i), &finding_id, Engine::Mgtg, FeedbackVerdict::Correct);
            store.store(entry).unwrap();
        }
        // Look up entries for a specific finding_id — 100 entries should match (10_000 / 100)
        let entries = store.get_by_finding_id("f0").unwrap();
        assert_eq!(entries.len(), 100);
    }

    // ── Edge cases ────────────────────────────────────────────────────

    #[test]
    fn test_edge_10000_char_notes() {
        let mut store = InMemoryFeedbackStore::new();
        let notes = "x".repeat(10_000);
        let entry = FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "tester")
            .with_notes(&notes);
        store.store(entry).unwrap();
        let all = store.get_all().unwrap();
        assert_eq!(all[0].notes.len(), 10_000);
    }

    #[test]
    fn test_edge_unicode_emoji_in_fields() {
        let mut store = InMemoryFeedbackStore::new();
        let reviewer = "révi€wer 🧑‍💻";
        let notes = "noté with ❤️ and 🚀 unicode ✅";
        let entry = FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, reviewer)
            .with_notes(notes);
        store.store(entry).unwrap();
        let all = store.get_all().unwrap();
        assert_eq!(all[0].reviewer, reviewer);
        assert_eq!(all[0].notes, notes);
    }

    #[test]
    fn test_edge_long_file_path() {
        let mut store = InMemoryFeedbackStore::new();
        let long_path = format!("{}/src/main.rs", "/a/b/c".repeat(200));
        assert!(long_path.len() > 1000);
        let entry = FeedbackEntry::new("e1", "f1", Engine::Mgtg, &long_path, 1, FeedbackVerdict::Correct, "tester");
        store.store(entry).unwrap();
        let all = store.get_all().unwrap();
        assert_eq!(all[0].file_path, long_path);
    }

    #[test]
    fn test_edge_line_zero() {
        let mut store = InMemoryFeedbackStore::new();
        let entry = FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 0, FeedbackVerdict::Correct, "tester");
        store.store(entry).unwrap();
        let all = store.get_all().unwrap();
        assert_eq!(all[0].line, 0);
    }

    #[test]
    fn test_edge_line_max_u32() {
        let mut store = InMemoryFeedbackStore::new();
        let entry = FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", u32::MAX, FeedbackVerdict::Correct, "tester");
        store.store(entry).unwrap();
        let all = store.get_all().unwrap();
        assert_eq!(all[0].line, u32::MAX);
    }

    #[test]
    fn test_edge_multiple_reviewers_same_finding() {
        let mut store = InMemoryFeedbackStore::new();
        for i in 0..5 {
            let entry = FeedbackEntry::new(
                &format!("e{}", i), "f1", Engine::Mgtg, "f.rs", 1,
                if i % 2 == 0 { FeedbackVerdict::Correct } else { FeedbackVerdict::Incorrect },
                &format!("reviewer{}", i),
            );
            store.store(entry).unwrap();
        }
        let entries = store.get_by_finding_id("f1").unwrap();
        assert_eq!(entries.len(), 5);
        // Each reviewer is unique
        let reviewers: std::collections::HashSet<&str> = entries.iter().map(|e| e.reviewer.as_str()).collect();
        assert_eq!(reviewers.len(), 5);

        let metrics = store.metrics().unwrap();
        assert_eq!(metrics.total_findings_with_feedback, 1);
        // 3 correct, 2 incorrect → 3/5 = 0.6 for mgtg
        let precision = metrics.precision_by_engine.get("mgtg").unwrap();
        assert!((*precision - 0.6).abs() < 1e-9);
    }
}
