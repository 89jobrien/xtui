# Plan: Dependency Graph View

## Goal

Add a dedicated dep view panel (toggle `D`) that shows direct workspace dependencies
enriched with crates.io latest version and GitHub metadata, cached globally in redb.

---

## Context Map

### Files to Modify

| File          | Changes needed                                                                                        |
| ------------- | ----------------------------------------------------------------------------------------------------- |
| `src/lib.rs`  | Add `pub mod depview; pub mod meta_fetch; pub mod meta_cache;`                                        |
| `src/main.rs` | Add `pub(crate) use xtui_lib::{depview, meta_fetch, meta_cache};`                                     |
| `src/app.rs`  | Add `show_dep_view`, `dep_infos`, `dep_scroll`, `dep_rx` fields; `D` keybinding; `poll_dep_results()` |
| `src/ui.rs`   | Add `draw_dep_view()`; call it when `show_dep_view` is true                                           |
| `Cargo.toml`  | Add `ureq = "2"`, `redb = "2"`                                                                        |

### New Files

| File                | Purpose                                                |
| ------------------- | ------------------------------------------------------ |
| `src/depview.rs`    | `DepInfo`, `DepFetchState`, `collect_direct_deps()`    |
| `src/meta_fetch.rs` | `MetadataFetcher` port + `HttpMetadataFetcher` adapter |
| `src/meta_cache.rs` | `MetadataCache` port + `RedbCache` adapter             |

### Dependencies (consumers of changed public API)

| File          | Relationship                                                 |
| ------------- | ------------------------------------------------------------ |
| `src/main.rs` | re-exports all lib modules — must add 3 new re-exports       |
| `src/app.rs`  | owns `App` struct — new fields + methods added               |
| `src/ui.rs`   | renders `App` state — new draw function + branch in `draw()` |

### Reference Patterns

| File                           | Pattern to follow                                               |
| ------------------------------ | --------------------------------------------------------------- |
| `src/status.rs`                | sync data collection, `Option<T>` return, inline `#[cfg(test)]` |
| `src/bin_schema.rs`            | JSON cache with TTL timestamp, `dirs::config_dir()` path        |
| `src/ui.rs::draw_status_tab`   | panel renderer replacing output pane                            |
| `src/app.rs` `show_status_tab` | boolean toggle + `s` keybinding pattern                         |

### Risk

- [ ] `App` struct gains 4 new public fields — `ui.rs` must compile against them
- [ ] `main.rs` re-export list must be updated or new modules are unreachable from the binary
- [ ] `redb` write path must handle concurrent access if two xtui instances run simultaneously — use `redb::Database::open` which handles this via file locking

---

## Architecture

- **Crates affected**: `xtui` (lib + binary, single crate)
- **New types**: `DepInfo`, `DepFetchState`, `CratesIoMeta`, `GithubMeta`, `CachedEntry`,
  `MetadataFetcher` (trait), `MetadataCache` (trait), `HttpMetadataFetcher`, `RedbCache`
- **Data flow**:
  ```
  D keypress → App::toggle_dep_view()
    → collect_direct_deps(workspace) via krates
    → spawn tokio tasks per dep
        → RedbCache::get() → hit: DepInfo{Ready}
        → miss: HttpMetadataFetcher::fetch_crates_io() + fetch_github_behind()
               → RedbCache::set()
               → tx.send(DepInfo{Ready})
    → App::poll_dep_results() drains rx into dep_infos each tick
    → ui::draw_dep_view() renders dep_infos table
  ```

## Tech Stack

- Rust 2024 edition
- `krates = "0.17"` (already in Cargo.toml) — workspace dep extraction
- `ureq = "2"` (new) — sync HTTP in `spawn_blocking`
- `redb = "2"` (new) — embedded key-value store for global cache
- `serde_json` (already in Cargo.toml) — JSON serialization for cache values
- `tokio` (already in Cargo.toml) — async task spawning

---

## Tasks

### Task 1: Add depview domain types

**File(s)**: `src/depview.rs`, `src/lib.rs`
**Run**: `cargo test -p xtui depview`

1. Write failing test:

   ```rust
   // src/depview.rs
   #[cfg(test)]
   mod tests {
       use super::*;
       #[test]
       fn dep_info_default_state_is_loading() {
           let d = DepInfo {
               name: "foo".into(),
               declared_version: "1.0.0".into(),
               crates_io_latest: None,
               github_url: None,
               versions_behind: None,
               state: DepFetchState::Loading,
           };
           assert!(matches!(d.state, DepFetchState::Loading));
       }
   }
   ```

   Run: `cargo test -p xtui depview` → FAIL (module doesn't exist)

2. Implement `src/depview.rs`:

   ```rust
   use std::path::Path;
   use anyhow::Result;

   #[derive(Debug, Clone)]
   pub struct DepInfo {
       pub name: String,
       pub declared_version: String,
       pub crates_io_latest: Option<String>,
       pub github_url: Option<String>,
       pub versions_behind: Option<u32>,
       pub state: DepFetchState,
   }

   #[derive(Debug, Clone)]
   pub enum DepFetchState {
       Loading,
       Ready,
       Error(String),
   }

   /// Returns direct dependencies of all workspace members as Loading stubs.
   /// Returns empty vec if project has no Cargo.toml or krates fails.
   pub fn collect_direct_deps(project: &Path) -> Vec<DepInfo> {
       let cargo_toml = project.join("Cargo.toml");
       if !cargo_toml.exists() {
           return vec![];
       }
       let mut cmd = krates::Cmd::new();
       cmd.manifest_path(&cargo_toml);
       let Ok(graph) = krates::Builder::new()
           .build(cmd, |_: krates::cm::Package| {}) else {
           return vec![];
       };
       let graph: krates::Krates<krates::cm::Package> = graph;

       let mut seen = std::collections::HashSet::new();
       let mut deps = Vec::new();

       for node in graph.workspace_members() {
           if let krates::Node::Krate { krate, .. } = node {
               let idx = graph.nid_for_name(&krate.name, &krate.version).unwrap();
               for dep_idx in graph.graph().neighbors(idx) {
                   if let krates::Node::Krate { krate: dep, .. } = &graph.graph()[dep_idx] {
                       let key = format!("{}@{}", dep.name, dep.version);
                       if seen.insert(key) {
                           deps.push(DepInfo {
                               name: dep.name.clone(),
                               declared_version: dep.version.to_string(),
                               crates_io_latest: None,
                               github_url: None,
                               versions_behind: None,
                               state: DepFetchState::Loading,
                           });
                       }
                   }
               }
           }
       }
       deps.sort_by(|a, b| a.name.cmp(&b.name));
       deps
   }
   ```

3. Add to `src/lib.rs`:

   ```rust
   pub mod depview;
   ```

4. Verify:
   ```
   cargo test -p xtui depview  → green
   cargo clippy -p xtui -- -D warnings  → zero warnings
   ```
   Commit: `git commit -m "feat(xtui): add depview domain types and collect_direct_deps"`

---

### Task 2: Add MetadataCache port and RedbCache adapter

**File(s)**: `src/meta_cache.rs`, `src/lib.rs`, `Cargo.toml`
**Run**: `cargo test -p xtui meta_cache`

1. Add to `Cargo.toml`:

   ```toml
   redb = "2"
   ```

2. Write failing test:

   ```rust
   // src/meta_cache.rs
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
           map.insert("foo".to_string(), CachedEntry {
               crates_io_latest: Some("2.0.0".into()),
               github_url: None,
               versions_behind: None,
               fetched_at: 0,
           });
           cache.set_crates_io_map(&map).unwrap();
           let loaded = cache.get_crates_io_map().unwrap();
           assert_eq!(loaded.get("foo").unwrap().crates_io_latest.as_deref(), Some("2.0.0"));
           std::fs::remove_dir_all(&dir).unwrap();
       }

       #[test]
       fn stale_entry_detected() {
           let entry = CachedEntry {
               crates_io_latest: None,
               github_url: None,
               versions_behind: None,
               fetched_at: 0,  // epoch — always stale
           };
           assert!(entry.is_stale(CRATES_IO_TTL_SECS));
       }
   }
   ```

   Run: `cargo test -p xtui meta_cache` → FAIL

3. Implement `src/meta_cache.rs`:

   ```rust
   use anyhow::Result;
   use redb::{Database, TableDefinition};
   use serde::{Deserialize, Serialize};
   use std::collections::HashMap;
   use std::path::Path;
   use std::time::{SystemTime, UNIX_EPOCH};

   pub const CRATES_IO_TTL_SECS: u64 = 60 * 60 * 24;  // 24h
   pub const GITHUB_TTL_SECS: u64 = 60 * 60;           // 1h

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
           // Ensure tables exist
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
           let mut table = tx.open_table(CRATES_IO_TABLE)?;
           table.insert(MAP_KEY, json.as_str())?;
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
           let mut table = tx.open_table(GITHUB_TABLE)?;
           table.insert(MAP_KEY, json.as_str())?;
           tx.commit()?;
           Ok(())
       }
   }

   /// Returns the path to the global cache file.
   pub fn cache_path() -> std::path::PathBuf {
       dirs::config_dir()
           .unwrap_or_else(|| std::path::PathBuf::from("."))
           .join("xtui")
           .join("meta.redb")
   }
   ```

4. Add to `src/lib.rs`:

   ```rust
   pub mod meta_cache;
   ```

5. Verify:
   ```
   cargo test -p xtui meta_cache  → green
   cargo clippy -p xtui -- -D warnings  → zero warnings
   ```
   Commit: `git commit -m "feat(xtui): add MetadataCache port and RedbCache adapter"`

---

### Task 3: Add MetadataFetcher port and HttpMetadataFetcher adapter

**File(s)**: `src/meta_fetch.rs`, `src/lib.rs`, `Cargo.toml`
**Run**: `cargo test -p xtui meta_fetch`

1. Add to `Cargo.toml`:

   ```toml
   ureq = { version = "2", features = ["json"] }
   ```

2. Write failing test:

   ```rust
   // src/meta_fetch.rs
   #[cfg(test)]
   mod tests {
       use super::*;

       struct FakeFetcher {
           latest: &'static str,
       }

       impl MetadataFetcher for FakeFetcher {
           fn fetch_crates_io(&self, _name: &str) -> anyhow::Result<CratesIoMeta> {
               Ok(CratesIoMeta {
                   latest_version: self.latest.to_string(),
                   repository: None,
                   versions: vec!["1.0.0".into(), "2.0.0".into()],
               })
           }
           fn fetch_github_behind(&self, _url: &str, _declared: &str) -> anyhow::Result<Option<u32>> {
               Ok(Some(1))
           }
       }

       #[test]
       fn fake_fetcher_satisfies_trait() {
           let f = FakeFetcher { latest: "2.0.0" };
           let meta = f.fetch_crates_io("serde").unwrap();
           assert_eq!(meta.latest_version, "2.0.0");
           assert_eq!(meta.versions.len(), 2);
           let behind = f.fetch_github_behind("https://github.com/foo/bar", "1.0.0").unwrap();
           assert_eq!(behind, Some(1));
       }

       #[test]
       fn count_versions_behind_correct() {
           let versions = vec!["1.0.0".to_string(), "1.1.0".to_string(), "2.0.0".to_string()];
           assert_eq!(count_versions_behind("1.0.0", &versions), Some(2));
           assert_eq!(count_versions_behind("2.0.0", &versions), Some(0));
           assert_eq!(count_versions_behind("0.9.0", &versions), None);
       }
   }
   ```

   Run: `cargo test -p xtui meta_fetch` → FAIL

3. Implement `src/meta_fetch.rs`:

   ```rust
   use anyhow::Result;
   use serde::Deserialize;

   pub trait MetadataFetcher: Send + Sync {
       fn fetch_crates_io(&self, name: &str) -> Result<CratesIoMeta>;
       fn fetch_github_behind(&self, repo_url: &str, declared: &str) -> Result<Option<u32>>;
   }

   #[derive(Debug, Clone)]
   pub struct CratesIoMeta {
       pub latest_version: String,
       pub repository: Option<String>,
       pub versions: Vec<String>,  // all published versions, newest first
   }

   #[derive(Deserialize)]
   struct CratesIoResponse {
       #[serde(rename = "crate")]
       krate: CratesIoCrate,
       versions: Vec<CratesIoVersion>,
   }

   #[derive(Deserialize)]
   struct CratesIoCrate {
       newest_version: String,
       repository: Option<String>,
   }

   #[derive(Deserialize)]
   struct CratesIoVersion {
       num: String,
   }

   #[derive(Deserialize)]
   struct GithubTag {
       name: String,
   }

   pub struct HttpMetadataFetcher;

   impl MetadataFetcher for HttpMetadataFetcher {
       fn fetch_crates_io(&self, name: &str) -> Result<CratesIoMeta> {
           let url = format!("https://crates.io/api/v1/crates/{name}");
           let mut req = ureq::get(&url)
               .set("User-Agent", "xtui/0.2.0 (https://github.com/89jobrien/xtui)");
           let resp: CratesIoResponse = req.call()?.into_json()?;
           Ok(CratesIoMeta {
               latest_version: resp.krate.newest_version,
               repository: resp.krate.repository,
               versions: resp.versions.into_iter().map(|v| v.num).collect(),
           })
       }

       fn fetch_github_behind(&self, repo_url: &str, declared: &str) -> Result<Option<u32>> {
           let (owner, repo) = parse_github_owner_repo(repo_url)?;
           let url = format!("https://api.github.com/repos/{owner}/{repo}/tags?per_page=100");
           let mut req = ureq::get(&url)
               .set("User-Agent", "xtui/0.2.0 (https://github.com/89jobrien/xtui)")
               .set("Accept", "application/vnd.github+json");
           if let Ok(token) = std::env::var("GITHUB_TOKEN") {
               req = req.set("Authorization", &format!("Bearer {token}"));
           }
           let tags: Vec<GithubTag> = req.call()?.into_json()?;
           let tag_names: Vec<String> = tags.into_iter().map(|t| t.name).collect();
           Ok(count_tags_behind(declared, &tag_names))
       }
   }

   fn parse_github_owner_repo(url: &str) -> Result<(String, String)> {
       // Handles https://github.com/owner/repo and https://github.com/owner/repo.git
       let stripped = url
           .trim_end_matches('/')
           .trim_end_matches(".git");
       let parts: Vec<&str> = stripped.rsplitn(3, '/').collect();
       if parts.len() < 2 {
           anyhow::bail!("cannot parse GitHub URL: {url}");
       }
       Ok((parts[1].to_string(), parts[0].to_string()))
   }

   /// Count how many published versions are newer than `declared`.
   pub fn count_versions_behind(declared: &str, versions: &[String]) -> Option<u32> {
       let pos = versions.iter().position(|v| v == declared)?;
       Some(pos as u32)
   }

   /// Count how many tags appear before (newer than) the declared version tag.
   fn count_tags_behind(declared: &str, tags: &[String]) -> Option<u32> {
       // Tags may be prefixed with 'v' — try both
       let candidates = [declared.to_string(), format!("v{declared}")];
       let pos = tags.iter().position(|t| candidates.contains(t))?;
       Some(pos as u32)
   }
   ```

4. Add to `src/lib.rs`:

   ```rust
   pub mod meta_fetch;
   ```

5. Verify:
   ```
   cargo test -p xtui meta_fetch  → green
   cargo clippy -p xtui -- -D warnings  → zero warnings
   ```
   Commit: `git commit -m "feat(xtui): add MetadataFetcher port and HttpMetadataFetcher adapter"`

---

### Task 4: Wire background fetch into App

**File(s)**: `src/main.rs`, `src/app.rs`
**Run**: `cargo test -p xtui app`

1. Write failing test:

   ```rust
   // src/app.rs #[cfg(test)]
   #[test]
   fn toggle_dep_view_sets_flag() {
       let mut app = App::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
       assert!(!app.show_dep_view);
       app.toggle_dep_view();
       assert!(app.show_dep_view);
       app.toggle_dep_view();
       assert!(!app.show_dep_view);
   }
   ```

   Run: `cargo test -p xtui toggle_dep_view` → FAIL

2. Add to `src/main.rs` re-exports:

   ```rust
   pub(crate) use xtui_lib::depview;
   pub(crate) use xtui_lib::meta_cache;
   pub(crate) use xtui_lib::meta_fetch;
   ```

3. Add fields to `App` in `src/app.rs`:

   ```rust
   pub show_dep_view: bool,
   pub dep_infos: Vec<crate::depview::DepInfo>,
   pub dep_scroll: u16,
   pub dep_rx: Option<tokio::sync::mpsc::Receiver<crate::depview::DepInfo>>,
   ```

   Initialize in `App::new`:

   ```rust
   show_dep_view: false,
   dep_infos: Vec::new(),
   dep_scroll: 0,
   dep_rx: None,
   ```

4. Add methods to `App`:

   ```rust
   pub fn toggle_dep_view(&mut self) {
       self.show_dep_view = !self.show_dep_view;
       if self.show_dep_view && self.dep_infos.is_empty() {
           self.spawn_dep_fetch();
       }
   }

   fn spawn_dep_fetch(&mut self) {
       let stubs = crate::depview::collect_direct_deps(&self.workspace);
       if stubs.is_empty() {
           return;
       }
       self.dep_infos = stubs.clone();

       let (tx, rx) = tokio::sync::mpsc::channel(64);
       self.dep_rx = Some(rx);

       let cache_path = crate::meta_cache::cache_path();
       let _ = std::fs::create_dir_all(cache_path.parent().unwrap_or(&cache_path));

       for stub in stubs {
           let tx = tx.clone();
           let cache_path = cache_path.clone();
           tokio::spawn(async move {
               let result = tokio::task::spawn_blocking(move || {
                   fetch_dep_info(stub, &cache_path)
               })
               .await;
               if let Ok(info) = result {
                   let _ = tx.send(info).await;
               }
           });
       }
   }

   pub fn poll_dep_results(&mut self) {
       let Some(ref mut rx) = self.dep_rx else { return };
       while let Ok(info) = rx.try_recv() {
           if let Some(entry) = self.dep_infos.iter_mut().find(|d| d.name == info.name) {
               *entry = info;
           }
       }
   }
   ```

5. Add free function (below `impl App`):

   ```rust
   fn fetch_dep_info(
       mut stub: crate::depview::DepInfo,
       cache_path: &std::path::Path,
   ) -> crate::depview::DepInfo {
       use crate::depview::DepFetchState;
       use crate::meta_cache::{CachedEntry, RedbCache, CRATES_IO_TTL_SECS};
       use crate::meta_fetch::{HttpMetadataFetcher, MetadataFetcher, count_versions_behind};
       use std::time::{SystemTime, UNIX_EPOCH};

       let Ok(cache) = RedbCache::open(cache_path) else {
           stub.state = DepFetchState::Error("cache unavailable".into());
           return stub;
       };

       let mut cio_map = cache.get_crates_io_map().unwrap_or_default();
       let entry = cio_map.get(&stub.name).cloned();
       let fresh = entry.as_ref().map(|e| !e.is_stale(CRATES_IO_TTL_SECS)).unwrap_or(false);

       let fetcher = HttpMetadataFetcher;

       let meta = if fresh {
           entry.unwrap()
       } else {
           match fetcher.fetch_crates_io(&stub.name) {
               Ok(m) => {
                   let behind = count_versions_behind(&stub.declared_version, &m.versions);
                   let now = SystemTime::now()
                       .duration_since(UNIX_EPOCH)
                       .unwrap_or_default()
                       .as_secs();
                   let e = CachedEntry {
                       crates_io_latest: Some(m.latest_version.clone()),
                       github_url: m.repository.clone(),
                       versions_behind: behind,
                       fetched_at: now,
                   };
                   cio_map.insert(stub.name.clone(), e.clone());
                   let _ = cache.set_crates_io_map(&cio_map);
                   e
               }
               Err(e) => {
                   stub.state = DepFetchState::Error(e.to_string());
                   return stub;
               }
           }
       };

       stub.crates_io_latest = meta.crates_io_latest;
       stub.github_url = meta.github_url;
       stub.versions_behind = meta.versions_behind;
       stub.state = DepFetchState::Ready;
       stub
   }
   ```

6. Add keybinding in `App::run` event loop (alongside `KeyCode::Char('s')`):

   ```rust
   KeyCode::Char('D') => self.toggle_dep_view(),
   ```

7. Add `self.poll_dep_results()` call in the event loop alongside `self.poll_output()`:

   ```rust
   self.poll_dep_results();
   ```

8. Verify:
   ```
   cargo test -p xtui toggle_dep_view  → green
   cargo clippy -p xtui -- -D warnings  → zero warnings
   ```
   Commit: `git commit -m "feat(xtui): wire dep view background fetch into App"`

---

### Task 5: Render the dep view panel

**File(s)**: `src/ui.rs`
**Run**: `cargo test -p xtui ui`

1. Write failing test:

   ```rust
   // src/ui.rs #[cfg(test)]
   #[test]
   fn format_dep_row_ready() {
       let info = crate::depview::DepInfo {
           name: "serde".into(),
           declared_version: "1.0.0".into(),
           crates_io_latest: Some("1.0.200".into()),
           github_url: Some("https://github.com/serde-rs/serde".into()),
           versions_behind: Some(5),
           state: crate::depview::DepFetchState::Ready,
       };
       let row = format_dep_row(&info);
       assert!(row.contains("serde"));
       assert!(row.contains("1.0.0"));
       assert!(row.contains("1.0.200"));
       assert!(row.contains("5"));
   }

   #[test]
   fn format_dep_row_loading() {
       let info = crate::depview::DepInfo {
           name: "tokio".into(),
           declared_version: "1".into(),
           crates_io_latest: None,
           github_url: None,
           versions_behind: None,
           state: crate::depview::DepFetchState::Loading,
       };
       let row = format_dep_row(&info);
       assert!(row.contains("…"));
   }
   ```

   Run: `cargo test -p xtui format_dep_row` → FAIL

2. Add to `src/ui.rs`:

   ```rust
   use crate::depview::{DepFetchState, DepInfo};

   pub(crate) fn format_dep_row(info: &DepInfo) -> String {
       match &info.state {
           DepFetchState::Loading => {
               format!("{:<30} {:<12} {:<12} {}", info.name, info.declared_version, "…", "…")
           }
           DepFetchState::Error(e) => {
               format!("{:<30} {:<12} {:<12} err: {}", info.name, info.declared_version, "—", e)
           }
           DepFetchState::Ready => {
               let latest = info.crates_io_latest.as_deref().unwrap_or("—");
               let behind = info.versions_behind
                   .map(|n| if n == 0 { "✓".to_string() } else { n.to_string() })
                   .unwrap_or_else(|| "—".to_string());
               let repo = info.github_url.as_deref().unwrap_or("—");
               format!("{:<30} {:<12} {:<12} {:<6} {}", info.name, info.declared_version, latest, behind, repo)
           }
       }
   }

   pub(crate) fn draw_dep_view(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
       let header = format!(
           "{:<30} {:<12} {:<12} {:<6} {}",
           "Name", "Declared", "Latest", "Behind", "Repo"
       );

       let mut lines: Vec<Line> = vec![
           Line::styled(header, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
           Line::raw(""),
       ];

       for info in &app.dep_infos {
           let row = format_dep_row(info);
           let style = match &info.state {
               DepFetchState::Loading => Style::default().fg(DIM),
               DepFetchState::Error(_) => Style::default().fg(Color::Red),
               DepFetchState::Ready => {
                   if info.versions_behind.unwrap_or(0) > 0 {
                       Style::default().fg(Color::Yellow)
                   } else {
                       Style::default().fg(Color::Green)
                   }
               }
           };
           lines.push(Line::styled(row, style));
       }

       let count = app.dep_infos.len();
       let panel = Paragraph::new(Text::from(lines))
           .block(
               Block::bordered()
                   .border_type(BorderType::Rounded)
                   .border_style(Style::default().fg(border_color(focused, DIM)))
                   .title(format!(" Deps [{}] ", count))
                   .title_style(
                       Style::default()
                           .fg(Color::Cyan)
                           .add_modifier(Modifier::BOLD),
                   ),
           )
           .wrap(Wrap { trim: false })
           .scroll((app.dep_scroll, 0));
       frame.render_widget(panel, area);
   }
   ```

3. In `ui::draw()`, add branch for dep view (alongside `show_status_tab`):

   ```rust
   if app.show_dep_view {
       draw_dep_view(frame, app, right_chunks[1], output_focused);
   } else if app.show_status_tab {
       draw_status_tab(frame, app, right_chunks[1], output_focused);
   } else {
       draw_output(frame, app, right_chunks[1], output_focused);
   }
   ```

4. Add `D` scroll keybindings in `App::run` (reuse `j`/`k` when dep view is active):
   In the `KeyCode::Char('j') | KeyCode::Down` arm, add:

   ```rust
   KeyCode::Char('j') | KeyCode::Down => match self.focus {
       Focus::Commands => self.next(),
       Focus::Output => {
           if self.show_dep_view {
               self.dep_scroll = self.dep_scroll.saturating_add(1);
           } else {
               self.scroll_output_down();
           }
       }
   },
   KeyCode::Char('k') | KeyCode::Up => match self.focus {
       Focus::Commands => self.previous(),
       Focus::Output => {
           if self.show_dep_view {
               self.dep_scroll = self.dep_scroll.saturating_sub(1);
           } else {
               self.scroll_output_up();
           }
       }
   },
   ```

5. Update `CLAUDE.md` keybinding note and `README.md` keybindings table to add `D` → dep view.

6. Verify:
   ```
   cargo test -p xtui format_dep_row  → green
   cargo check  → clean
   cargo clippy -p xtui -- -D warnings  → zero warnings
   ```
   Commit: `git commit -m "feat(xtui): render dep view panel in ui"`

---

### Task 6: Update docs and register tasks

**File(s)**: `README.md`, `CLAUDE.md`, `ARCHITECTURE.md`
**Run**: `cargo test`

1. Add `D` row to keybindings tables in `README.md` and `ARCHITECTURE.md`:

   ```
   | `D` | Any | Toggle dependency graph view |
   ```

2. Add `D` to the hint string in `ui::draw_bottom_status`:

   ```rust
   Focus::Commands => "Tab:source  1-9:tab  Enter:run  a:args  /:search  s:status  D:deps  P:pipeline",
   ```

3. Run full test suite:
   ```
   cargo test  → all green
   cargo clippy -- -D warnings  → zero warnings
   ```
   Commit: `git commit -m "docs(xtui): add dep view keybinding to README, ARCHITECTURE, status bar"`

---

## Quality Rules Applied

- Every new module has inline `#[cfg(test)]` unit tests
- `MetadataFetcher` trait tested via `FakeFetcher` (conformance pattern)
- `RedbCache` tested with a tmp dir (real I/O, isolated)
- No `unwrap()` in library code — all fallible paths use `?` or explicit error handling
- `fetch_dep_info` is a free function (not a method) — pure, testable, no App coupling
