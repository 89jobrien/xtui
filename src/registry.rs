use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    pub path: PathBuf,
    pub pinned: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectCache {
    pub projects: Vec<ProjectEntry>,
    pub last_scan: String,
}

const MARKERS: &[&str] = &[
    "Cargo.toml",
    "Justfile",
    "justfile",
    "package.json",
    "Makefile",
    "mise.toml",
    ".mise.toml",
];

fn has_marker(dir: &Path) -> bool {
    MARKERS.iter().any(|m| dir.join(m).exists()) || dir.join("xtask").is_dir()
}

/// Scans one level of `base` for subdirectories that contain a project marker.
pub fn scan_directory(base: &Path) -> Vec<ProjectEntry> {
    let Ok(entries) = std::fs::read_dir(base) else {
        return vec![];
    };

    let mut projects: Vec<ProjectEntry> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_dir() && has_marker(&path) {
                Some(ProjectEntry {
                    path,
                    pinned: false,
                })
            } else {
                None
            }
        })
        .collect();

    projects.sort_by(|a, b| a.path.cmp(&b.path));
    projects
}

/// Returns `~/.config/xtui/`.
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("xtui")
}

/// Reads `~/.config/xtui/projects.json`, returning `None` on any error.
pub fn load_cache() -> Option<ProjectCache> {
    let path = config_dir().join("projects.json");
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Writes `cache` as JSON to `~/.config/xtui/projects.json`.
pub fn save_cache(cache: &ProjectCache) -> Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("projects.json");
    let text = serde_json::to_string_pretty(cache)?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Toggles the `pinned` flag for the entry matching `path`.
/// If no entry matches, this is a no-op.
pub fn toggle_pin(cache: &mut ProjectCache, path: &Path) {
    for entry in &mut cache.projects {
        if entry.path == path {
            entry.pinned = !entry.pinned;
            return;
        }
    }
}

/// Returns references to projects sorted: pinned first, then alphabetical by path.
pub fn sorted_projects(cache: &ProjectCache) -> Vec<&ProjectEntry> {
    let mut refs: Vec<&ProjectEntry> = cache.projects.iter().collect();
    refs.sort_by(|a, b| b.pinned.cmp(&a.pinned).then_with(|| a.path.cmp(&b.path)));
    refs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(suffix: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("xtui-registry-{suffix}"));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn scan_nonexistent_path() {
        let projects = scan_directory(std::path::Path::new("/nonexistent/path/xtui-test"));
        assert!(projects.is_empty());
    }

    #[test]
    fn scan_finds_dirs_with_cargo_toml() {
        let base = temp_dir("cargo");
        let proj = base.join("myproj");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("Cargo.toml"), "[package]\nname=\"x\"").unwrap();

        let projects = scan_directory(&base);
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].path, proj);
        assert!(!projects[0].pinned);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn scan_finds_dirs_with_justfile() {
        let base = temp_dir("just");
        let proj = base.join("just-proj");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("Justfile"), "build:\n  cargo build\n").unwrap();

        let projects = scan_directory(&base);
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].path, proj);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn scan_finds_dirs_with_xtask_subdir() {
        let base = temp_dir("xtask");
        let proj = base.join("xtask-proj");
        fs::create_dir_all(proj.join("xtask")).unwrap();

        let projects = scan_directory(&base);
        assert_eq!(projects.len(), 1);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn scan_ignores_dirs_without_markers() {
        let base = temp_dir("empty");
        fs::create_dir_all(base.join("not-a-project")).unwrap();

        let projects = scan_directory(&base);
        assert!(projects.is_empty());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn scan_returns_sorted_results() {
        let base = temp_dir("sorted");
        for name in &["zzz", "aaa", "mmm"] {
            let p = base.join(name);
            fs::create_dir_all(&p).unwrap();
            fs::write(p.join("Cargo.toml"), "").unwrap();
        }

        let projects = scan_directory(&base);
        assert_eq!(projects.len(), 3);
        assert!(projects[0].path.ends_with("aaa"));
        assert!(projects[1].path.ends_with("mmm"));
        assert!(projects[2].path.ends_with("zzz"));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn toggle_pin_flips_flag() {
        let mut cache = ProjectCache {
            projects: vec![
                ProjectEntry {
                    path: PathBuf::from("/a"),
                    pinned: false,
                },
                ProjectEntry {
                    path: PathBuf::from("/b"),
                    pinned: true,
                },
            ],
            last_scan: String::new(),
        };

        toggle_pin(&mut cache, Path::new("/a"));
        assert!(cache.projects[0].pinned);

        toggle_pin(&mut cache, Path::new("/a"));
        assert!(!cache.projects[0].pinned);

        toggle_pin(&mut cache, Path::new("/b"));
        assert!(!cache.projects[1].pinned);
    }

    #[test]
    fn toggle_pin_noop_for_unknown_path() {
        let mut cache = ProjectCache {
            projects: vec![ProjectEntry {
                path: PathBuf::from("/a"),
                pinned: false,
            }],
            last_scan: String::new(),
        };
        toggle_pin(&mut cache, Path::new("/nonexistent"));
        assert!(!cache.projects[0].pinned);
    }

    #[test]
    fn sorted_projects_pinned_first_then_alpha() {
        let cache = ProjectCache {
            projects: vec![
                ProjectEntry {
                    path: PathBuf::from("/c"),
                    pinned: false,
                },
                ProjectEntry {
                    path: PathBuf::from("/a"),
                    pinned: true,
                },
                ProjectEntry {
                    path: PathBuf::from("/b"),
                    pinned: false,
                },
                ProjectEntry {
                    path: PathBuf::from("/d"),
                    pinned: true,
                },
            ],
            last_scan: String::new(),
        };

        let sorted = sorted_projects(&cache);
        assert_eq!(sorted.len(), 4);
        assert!(sorted[0].pinned);
        assert!(sorted[1].pinned);
        assert_eq!(sorted[0].path, PathBuf::from("/a"));
        assert_eq!(sorted[1].path, PathBuf::from("/d"));
        assert!(!sorted[2].pinned);
        assert!(!sorted[3].pinned);
        assert_eq!(sorted[2].path, PathBuf::from("/b"));
        assert_eq!(sorted[3].path, PathBuf::from("/c"));
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = temp_dir("roundtrip");
        let cache_file = dir.join("projects.json");

        let cache = ProjectCache {
            projects: vec![
                ProjectEntry {
                    path: PathBuf::from("/x"),
                    pinned: true,
                },
                ProjectEntry {
                    path: PathBuf::from("/y"),
                    pinned: false,
                },
            ],
            last_scan: "2026-06-08T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string_pretty(&cache).unwrap();
        fs::write(&cache_file, json).unwrap();

        let loaded: ProjectCache =
            serde_json::from_str(&fs::read_to_string(&cache_file).unwrap()).unwrap();

        assert_eq!(loaded.projects.len(), 2);
        assert_eq!(loaded.projects[0].path, PathBuf::from("/x"));
        assert!(loaded.projects[0].pinned);
        assert_eq!(loaded.last_scan, "2026-06-08T00:00:00Z");

        let _ = fs::remove_dir_all(&dir);
    }
}
