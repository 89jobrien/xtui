use anyhow::Result;
use redb::{Database, TableDefinition};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const CRATES_IO_TTL_SECS: u64 = 60 * 60 * 24; // 24h
pub const GITHUB_TTL_SECS: u64 = 60 * 60; // 1h

const CRATES_IO_TABLE: TableDefinition<&str, &str> = TableDefinition::new("crates_io");
const GITHUB_TABLE: TableDefinition<&str, &str> = TableDefinition::new("github");
const MAP_KEY: &str = "map";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedEntry {
    pub crates_io_latest: Option<String>,
    pub github_url: Option<String>,
    pub versions_behind: Option<u32>,
    pub fetched_at: u64,
}

impl CachedEntry {
    pub fn is_stale(&self, ttl_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.fetched_at) > ttl_secs
    }
}

pub struct RedbCache {
    db: Database,
}

impl RedbCache {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = Database::create(path)?;
        let tx = db.begin_write()?;
        tx.open_table(CRATES_IO_TABLE)?;
        tx.open_table(GITHUB_TABLE)?;
        tx.commit()?;
        Ok(Self { db })
    }

    pub fn get_crates_io_map(&self) -> Result<HashMap<String, CachedEntry>> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(CRATES_IO_TABLE)?;
        let json = table.get(MAP_KEY)?.map(|v| v.value().to_owned());
        match json {
            Some(s) => Ok(serde_json::from_str(&s)?),
            None => Ok(HashMap::new()),
        }
    }

    pub fn set_crates_io_map(&self, map: &HashMap<String, CachedEntry>) -> Result<()> {
        let json = serde_json::to_string(map)?;
        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(CRATES_IO_TABLE)?;
            table.insert(MAP_KEY, json.as_str())?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_github_map(&self) -> Result<HashMap<String, CachedEntry>> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(GITHUB_TABLE)?;
        let json = table.get(MAP_KEY)?.map(|v| v.value().to_owned());
        match json {
            Some(s) => Ok(serde_json::from_str(&s)?),
            None => Ok(HashMap::new()),
        }
    }

    pub fn set_github_map(&self, map: &HashMap<String, CachedEntry>) -> Result<()> {
        let json = serde_json::to_string(map)?;
        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(GITHUB_TABLE)?;
            table.insert(MAP_KEY, json.as_str())?;
        }
        tx.commit()?;
        Ok(())
    }
}

/// Returns the path to the global metadata cache file.
pub fn cache_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("xtui")
        .join("meta.redb")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn redb_cache_roundtrip() {
        let dir = std::env::temp_dir().join("xtui-test-cache");
        std::fs::create_dir_all(&dir).unwrap();
        let cache = RedbCache::open(dir.join("test.redb")).unwrap();
        let mut map = HashMap::new();
        map.insert(
            "foo".to_string(),
            CachedEntry {
                crates_io_latest: Some("2.0.0".into()),
                github_url: None,
                versions_behind: None,
                fetched_at: 0,
            },
        );
        cache.set_crates_io_map(&map).unwrap();
        let loaded = cache.get_crates_io_map().unwrap();
        assert_eq!(
            loaded.get("foo").unwrap().crates_io_latest.as_deref(),
            Some("2.0.0")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn stale_entry_detected() {
        let entry = CachedEntry {
            crates_io_latest: None,
            github_url: None,
            versions_behind: None,
            fetched_at: 0, // epoch — always stale
        };
        assert!(entry.is_stale(CRATES_IO_TTL_SECS));
    }

    #[test]
    fn fresh_entry_not_stale() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let entry = CachedEntry {
            crates_io_latest: None,
            github_url: None,
            versions_behind: None,
            fetched_at: now,
        };
        assert!(!entry.is_stale(CRATES_IO_TTL_SECS));
    }
}
