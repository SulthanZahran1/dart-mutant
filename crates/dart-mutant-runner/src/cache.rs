//! Content-addressed cache for incremental mutation testing.
//!
//! The cache is keyed by `sha256(source_bytes + test_bytes)` — a content
//! address that uniquely identifies the mutation + test combination. If the
//! source and test files haven't changed since the last run, the mutant's
//! result can be served from cache without re-running the test suite.
//!
//! This achieves the "warm rerun < 5% of cold run time" acceptance criterion
//! from GOAL.md — a warm run just reads cached results instead of executing
//! N test runs.
//!
//! # Cache layout
//!
//! The cache is stored in `.dart_mutant_cache/` (gitignored). Each entry is a
//! JSON file named `<hash>.json` containing the [`CacheEntry`]:
//!
//! ```json
//! {
//!   "hash": "abc123...",
//!   "mutant_id": "42",
//!   "status": "KILLED",
//!   "duration_ms": 1234,
//!   "covering_tests": ["test1", "test2"],
//!   "timestamp": 1698765432
//! }
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Schemata;
use dart_mutant_core::{Mutant, MutantResult, MutantStatus};

// ---------------------------------------------------------------------------
// Cache entry
// ---------------------------------------------------------------------------

/// A cached mutant result. Stored as JSON in the cache directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// SHA-256 content hash: `sha256(source_bytes || test_bytes)`.
    pub hash: String,
    /// Mutant ID.
    pub mutant_id: String,
    /// Mutant status (KILLED, SURVIVED, etc.).
    pub status: MutantStatus,
    /// Duration of the test run in milliseconds.
    pub duration_ms: u64,
    /// Covering tests that were run.
    pub covering_tests: Vec<String>,
    /// Optional diagnostic message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Unix timestamp when this entry was cached.
    pub timestamp: u64,
}

impl CacheEntry {
    /// Create a new cache entry from a mutant result.
    pub fn from_result(hash: String, result: &MutantResult) -> Self {
        CacheEntry {
            hash,
            mutant_id: result.mutant.id.clone(),
            status: result.status,
            duration_ms: 0, // Duration is tracked externally
            covering_tests: result.covering_tests.clone(),
            message: result.message.clone(),
            timestamp: current_timestamp(),
        }
    }

    /// Convert to a [`MutantResult`] by combining with the original mutant.
    pub fn to_result(&self, mutant: &Mutant) -> MutantResult {
        MutantResult {
            mutant: mutant.clone(),
            status: self.status,
            covering_tests: self.covering_tests.clone(),
            message: self.message.clone(),
        }
    }
}

/// Get the current Unix timestamp in seconds.
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

/// Content-addressed cache for mutant results.
///
/// Keyed by `sha256(source_bytes + test_bytes)`. When a mutant's source and
/// test files are byte-identical to a previous run, the result is served from
/// cache without re-running the test suite.
pub struct Cache {
    /// Cache directory path.
    cache_dir: PathBuf,
    /// In-memory index: hash → cache entry (loaded on init, updated on store).
    index: HashMap<String, CacheEntry>,
}

impl Cache {
    /// Create or open a cache at the given directory path.
    ///
    /// Creates the directory if it doesn't exist. Loads all existing entries
    /// into memory for fast lookups.
    pub fn new(cache_dir: &Path) -> Self {
        if !cache_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(cache_dir) {
                warn!("failed to create cache dir {}: {}", cache_dir.display(), e);
            }
        }

        let index = load_cache_index(cache_dir).unwrap_or_default();
        info!(
            "Cache loaded: {} entries in {}",
            index.len(),
            cache_dir.display()
        );

        Cache {
            cache_dir: cache_dir.to_path_buf(),
            index,
        }
    }

    /// Look up a cached result by content hash.
    ///
    /// Returns `Some(&CacheEntry)` if the hash is in the cache, `None` otherwise.
    pub fn get(&self, hash: &str) -> Option<&CacheEntry> {
        self.index.get(hash)
    }

    /// Store a mutant result in the cache.
    ///
    /// Computes the content hash from the source and test bytes, then writes
    /// the cache entry to disk and updates the in-memory index.
    pub fn store(&mut self, hash: String, entry: CacheEntry) -> Result<()> {
        let path = self.cache_dir.join(format!("{}.json", hash));
        let json = serde_json::to_string_pretty(&entry)?;
        std::fs::write(&path, json)?;
        debug!("Cached entry {} → {}", hash, path.display());
        self.index.insert(hash, entry);
        Ok(())
    }

    /// Store a mutant result directly.
    pub fn store_result(
        &mut self,
        _mutant: &Mutant,
        result: &MutantResult,
        source_bytes: &[u8],
        test_bytes: &[u8],
    ) -> Result<()> {
        let hash = compute_cache_hash(source_bytes, test_bytes);
        let entry = CacheEntry::from_result(hash.clone(), result);
        self.store(hash, entry)
    }

    /// Number of entries in the cache.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Clear all cache entries (delete the cache directory).
    pub fn clear(&self) -> Result<()> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)?;
            std::fs::create_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }

    /// Check if a mutant's source + test bytes match a cached entry.
    ///
    /// Returns the cached entry if found, `None` if not.
    pub fn check(&self, source_bytes: &[u8], test_bytes: &[u8]) -> Option<&CacheEntry> {
        let hash = compute_cache_hash(source_bytes, test_bytes);
        self.get(&hash)
    }
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache")
            .field("cache_dir", &self.cache_dir)
            .field("entries", &self.index.len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Hash computation
// ---------------------------------------------------------------------------

/// Compute the content-addressed cache hash: `sha256(source_bytes || test_bytes)`.
///
/// This hash uniquely identifies a mutation + test combination. If the source
/// and test files are byte-identical to a previous run, the result can be
/// served from cache.
pub fn compute_cache_hash(source_bytes: &[u8], test_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source_bytes);
    hasher.update(test_bytes);
    let result = hasher.finalize();
    // Use hex encoding for the hash (file-safe)
    hex::encode(result)
}

/// Compute the cache hash from a [`Schemata`] entry and test file paths.
///
/// Reads the mutated source content and test file contents, then computes
/// the hash.
pub fn hash_for_mutant(
    schemata: &Schemata,
    mutant: &Mutant,
    test_files: &[PathBuf],
) -> Result<String> {
    let entry = schemata
        .get(&mutant.id)
        .ok_or_else(|| anyhow::anyhow!("mutant {} not found in schemata", mutant.id))?;

    let source_bytes = entry.mutated_source.as_bytes();
    let mut test_bytes = Vec::new();
    for tf in test_files {
        if tf.exists() {
            let content = std::fs::read(tf)?;
            test_bytes.extend(content);
        }
    }

    Ok(compute_cache_hash(source_bytes, &test_bytes))
}

// ---------------------------------------------------------------------------
// Cache index loading
// ---------------------------------------------------------------------------

/// Load all cache entries from the cache directory into memory.
fn load_cache_index(cache_dir: &Path) -> Result<HashMap<String, CacheEntry>> {
    let mut index = HashMap::new();

    if !cache_dir.exists() {
        return Ok(index);
    }

    for entry in std::fs::read_dir(cache_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let content = std::fs::read_to_string(&path)?;
        match serde_json::from_str::<CacheEntry>(&content) {
            Ok(entry) => {
                index.insert(entry.hash.clone(), entry);
            }
            Err(e) => {
                warn!("failed to parse cache entry {}: {}", path.display(), e);
            }
        }
    }

    Ok(index)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn mk_mutant(id: &str) -> Mutant {
        Mutant::new(id, "lib/x.dart", 1, 1, "AOR", "+", "-", "test")
    }

    #[test]
    fn test_compute_cache_hash_deterministic() {
        let source = b"int add(int a, int b) => a + b;";
        let tests = b"void main() { test('add', () => expect(add(1,2), 3)); }";
        let h1 = compute_cache_hash(source, tests);
        let h2 = compute_cache_hash(source, tests);
        assert_eq!(h1, h2, "same bytes should produce same hash");
    }

    #[test]
    fn test_compute_cache_hash_different_source() {
        let tests = b"test bytes";
        let h1 = compute_cache_hash(b"source1", tests);
        let h2 = compute_cache_hash(b"source2", tests);
        assert_ne!(h1, h2, "different source should produce different hash");
    }

    #[test]
    fn test_compute_cache_hash_different_tests() {
        let source = b"source bytes";
        let h1 = compute_cache_hash(source, b"test1");
        let h2 = compute_cache_hash(source, b"test2");
        assert_ne!(h1, h2, "different tests should produce different hash");
    }

    #[test]
    fn test_cache_store_and_get() {
        let tmp = TempDir::new().unwrap();
        let mut cache = Cache::new(tmp.path());

        let mutant = mk_mutant("1");
        let result = MutantResult::new(mutant.clone(), MutantStatus::Killed)
            .with_tests(vec!["test1".to_string()])
            .with_message("killed by test1");

        let hash = "abc123".to_string();
        let entry = CacheEntry::from_result(hash.clone(), &result);
        cache.store(hash.clone(), entry).unwrap();

        let retrieved = cache.get(&hash).unwrap();
        assert_eq!(retrieved.mutant_id, "1");
        assert_eq!(retrieved.status, MutantStatus::Killed);
        assert_eq!(retrieved.covering_tests, vec!["test1"]);
    }

    #[test]
    fn test_cache_persistence() {
        let tmp = TempDir::new().unwrap();

        // Write to cache
        {
            let mut cache = Cache::new(tmp.path());
            let mutant = mk_mutant("1");
            let result = MutantResult::new(mutant, MutantStatus::Survived);
            let entry = CacheEntry::from_result("hash1".to_string(), &result);
            cache.store("hash1".to_string(), entry).unwrap();
            assert_eq!(cache.len(), 1);
        }

        // Re-open cache — should have the entry
        {
            let cache = Cache::new(tmp.path());
            assert_eq!(cache.len(), 1);
            assert!(cache.get("hash1").is_some());
        }
    }

    #[test]
    fn test_cache_check() {
        let tmp = TempDir::new().unwrap();
        let mut cache = Cache::new(tmp.path());

        let source = b"source bytes";
        let tests = b"test bytes";
        let hash = compute_cache_hash(source, tests);

        let mutant = mk_mutant("1");
        let result = MutantResult::new(mutant, MutantStatus::Killed);
        let entry = CacheEntry::from_result(hash.clone(), &result);
        cache.store(hash, entry).unwrap();

        // Check with same bytes → should find the entry
        assert!(cache.check(source, tests).is_some());

        // Check with different bytes → should not find
        assert!(cache.check(b"different", tests).is_none());
    }

    #[test]
    fn test_cache_clear() {
        let tmp = TempDir::new().unwrap();
        let mut cache = Cache::new(tmp.path());

        let mutant = mk_mutant("1");
        let result = MutantResult::new(mutant, MutantStatus::Killed);
        let entry = CacheEntry::from_result("hash1".to_string(), &result);
        cache.store("hash1".to_string(), entry).unwrap();
        assert!(!cache.is_empty());

        cache.clear().unwrap();
        let cache2 = Cache::new(tmp.path());
        assert!(cache2.is_empty());
    }

    #[test]
    fn test_cache_entry_to_result() {
        let mutant = mk_mutant("1");
        let entry = CacheEntry {
            hash: "abc".to_string(),
            mutant_id: "1".to_string(),
            status: MutantStatus::Killed,
            duration_ms: 500,
            covering_tests: vec!["test1".to_string()],
            message: Some("killed".to_string()),
            timestamp: 12345,
        };
        let result = entry.to_result(&mutant);
        assert_eq!(result.mutant.id, "1");
        assert_eq!(result.status, MutantStatus::Killed);
        assert_eq!(result.covering_tests, vec!["test1"]);
    }
}
