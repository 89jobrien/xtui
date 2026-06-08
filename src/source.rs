use std::path::Path;

use anyhow::Result;

use crate::discover::parse_source;

/// A command discovered from a project's xtask or similar source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCommand {
    pub name: String,
    pub description: Option<String>,
    pub source: String,
}

/// Port: any backend capable of discovering commands for a project.
pub trait CommandSource: Send + Sync {
    fn name(&self) -> &str;
    fn discover(&self, project: &Path) -> Result<Vec<SourceCommand>>;
}

/// Returns all built-in command sources.
pub fn all_sources() -> Vec<Box<dyn CommandSource>> {
    vec![
        Box::new(XtaskSource),
        Box::new(CargoSource),
        Box::new(JustSource),
        Box::new(NuScriptSource),
        Box::new(NpmSource),
        Box::new(MakeSource),
        Box::new(MiseSource),
    ]
}

// ── XtaskSource ──────────────────────────────────────────────────────────────

pub struct XtaskSource;

impl CommandSource for XtaskSource {
    fn name(&self) -> &str {
        "xtask"
    }

    fn discover(&self, project: &Path) -> Result<Vec<SourceCommand>> {
        let main_path = project.join("xtask/src/main.rs");
        if !main_path.exists() {
            return Ok(vec![]);
        }
        let src = std::fs::read_to_string(&main_path)?;
        let cmds = parse_source(&src)
            .into_iter()
            .map(|c| SourceCommand {
                name: c.name,
                description: c.description,
                source: "xtask".to_string(),
            })
            .collect();
        Ok(cmds)
    }
}

// ── CargoSource ──────────────────────────────────────────────────────────────

pub struct CargoSource;

impl CommandSource for CargoSource {
    fn name(&self) -> &str {
        "cargo"
    }

    // qual:allow(iosp,dry) reason: "I/O boundary — filesystem check + TOML read + data construction are inseparable; inline struct construction is intentional"
    fn discover(&self, project: &Path) -> Result<Vec<SourceCommand>> {
        if !project.join("Cargo.toml").exists() {
            return Ok(vec![]);
        }
        let fixed = ["check", "build", "test", "clippy"];
        let mut cmds: Vec<SourceCommand> = fixed
            .iter()
            .map(|&name| SourceCommand {
                name: name.to_string(),
                description: Some(format!("cargo {name}")),
                source: "cargo".to_string(),
            })
            .collect();

        // Parse [[bin]] entries from Cargo.toml for extra run targets
        if let Ok(contents) = std::fs::read_to_string(project.join("Cargo.toml"))
            && let Ok(doc) = contents.parse::<toml::Table>()
            && let Some(bins) = doc.get("bin").and_then(|v| v.as_array())
        {
            for bin in bins {
                if let Some(name) = bin.get("name").and_then(|v| v.as_str()) {
                    cmds.push(SourceCommand {
                        name: format!("run --bin {name}"),
                        description: Some(format!("cargo run --bin {name}")),
                        source: "cargo".to_string(),
                    });
                }
            }
        }

        Ok(cmds)
    }
}

// ── JustSource ───────────────────────────────────────────────────────────────

pub struct JustSource;

impl CommandSource for JustSource {
    fn name(&self) -> &str {
        "just"
    }

    fn discover(&self, project: &Path) -> Result<Vec<SourceCommand>> {
        let has_justfile = project.join("Justfile").exists() || project.join("justfile").exists();
        if !has_justfile {
            return Ok(vec![]);
        }

        let output = std::process::Command::new("just")
            .args(["--list", "--unsorted", "--list-heading=", "--list-prefix="])
            .current_dir(project)
            .output();

        let output = match output {
            Ok(o) if o.status.success() => o,
            _ => return Ok(vec![]),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let cmds = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let mut parts = line.splitn(2, '#');
                let name = parts.next().unwrap_or("").trim().to_string();
                let desc = parts.next().map(|s| s.trim().to_string());
                SourceCommand {
                    name,
                    description: desc,
                    source: "just".to_string(),
                }
            })
            .filter(|cmd| !cmd.name.is_empty())
            .collect();

        Ok(cmds)
    }
}

// ── NuScriptSource ───────────────────────────────────────────────────────────

pub struct NuScriptSource;

impl CommandSource for NuScriptSource {
    fn name(&self) -> &str {
        "nu"
    }

    fn discover(&self, project: &Path) -> Result<Vec<SourceCommand>> {
        let scripts_dir = project.join("scripts");
        if !scripts_dir.is_dir() {
            return Ok(vec![]);
        }

        let mut cmds = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&scripts_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("nu")
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                {
                    cmds.push(SourceCommand {
                        name: stem.to_string(),
                        description: Some(format!("nu scripts/{stem}.nu")),
                        source: "nu".to_string(),
                    });
                }
            }
        }
        cmds.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(cmds)
    }
}

// ── NpmSource ────────────────────────────────────────────────────────────────

pub struct NpmSource;

impl CommandSource for NpmSource {
    fn name(&self) -> &str {
        "npm"
    }

    fn discover(&self, project: &Path) -> Result<Vec<SourceCommand>> {
        let pkg_path = project.join("package.json");
        if !pkg_path.exists() {
            return Ok(vec![]);
        }

        let contents = std::fs::read_to_string(&pkg_path)?;
        let json: serde_json::Value = serde_json::from_str(&contents)?;

        let Some(scripts) = json.get("scripts").and_then(|s| s.as_object()) else {
            return Ok(vec![]);
        };

        let mut cmds: Vec<SourceCommand> = scripts
            .iter()
            .map(|(name, val)| SourceCommand {
                name: name.clone(),
                description: val.as_str().map(|s| s.to_string()),
                source: "npm".to_string(),
            })
            .collect();
        cmds.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(cmds)
    }
}

// ── MakeSource ───────────────────────────────────────────────────────────────

pub struct MakeSource;

impl CommandSource for MakeSource {
    fn name(&self) -> &str {
        "make"
    }

    fn discover(&self, project: &Path) -> Result<Vec<SourceCommand>> {
        let makefile = project.join("Makefile");
        if !makefile.exists() {
            return Ok(vec![]);
        }

        let contents = std::fs::read_to_string(&makefile)?;
        let re = regex::Regex::new(r"(?m)^([a-zA-Z_][a-zA-Z0-9_-]*)\s*:")?;

        let mut seen = std::collections::HashSet::new();
        let mut cmds = Vec::new();
        for caps in re.captures_iter(&contents) {
            let name = caps[1].to_string();
            if seen.insert(name.clone()) {
                cmds.push(SourceCommand {
                    name,
                    description: None,
                    source: "make".to_string(),
                });
            }
        }
        Ok(cmds)
    }
}

// ── MiseSource ───────────────────────────────────────────────────────────────

pub struct MiseSource;

impl CommandSource for MiseSource {
    fn name(&self) -> &str {
        "mise"
    }

    // qual:allow(iosp) reason: "I/O boundary — config file detection + TOML parse + data construction are inseparable"
    fn discover(&self, project: &Path) -> Result<Vec<SourceCommand>> {
        let mise_path = if project.join("mise.toml").exists() {
            project.join("mise.toml")
        } else if project.join(".mise.toml").exists() {
            project.join(".mise.toml")
        } else {
            return Ok(vec![]);
        };

        let contents = std::fs::read_to_string(&mise_path)?;
        let doc: toml::Table = contents.parse()?;

        let Some(tasks) = doc.get("tasks").and_then(|v| v.as_table()) else {
            return Ok(vec![]);
        };

        let mut cmds: Vec<SourceCommand> = tasks
            .keys()
            .map(|name| {
                let desc = tasks
                    .get(name)
                    .and_then(|v| v.as_table())
                    .and_then(|t| t.get("description"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                SourceCommand {
                    name: name.clone(),
                    description: desc,
                    source: "mise".to_string(),
                }
            })
            .collect();
        cmds.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(cmds)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn xtask_source_detects_xtask_dir() {
        let src = XtaskSource;
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let cmds = src.discover(root).unwrap();
        assert!(!cmds.is_empty());
        assert!(cmds.iter().any(|c| c.name == "check"));
    }

    #[test]
    fn xtask_source_returns_empty_for_non_xtask_dir() {
        let src = XtaskSource;
        let cmds = src.discover(Path::new("/tmp")).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn xtask_source_name_is_xtask() {
        assert_eq!(XtaskSource.name(), "xtask");
    }

    #[test]
    fn cargo_source_finds_cargo_toml() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let cmds = CargoSource.discover(root).unwrap();
        assert!(cmds.iter().any(|c| c.name == "check"));
        assert!(cmds.iter().any(|c| c.name == "test"));
        assert!(cmds.iter().all(|c| c.source == "cargo"));
    }

    #[test]
    fn cargo_source_empty_without_cargo_toml() {
        let cmds = CargoSource.discover(Path::new("/tmp")).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn nu_source_empty_without_scripts_dir() {
        let cmds = NuScriptSource.discover(Path::new("/tmp")).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn nu_source_finds_nu_scripts() {
        let tmp = std::env::temp_dir().join("xtui-test-nu-source");
        let _ = fs::remove_dir_all(&tmp);
        let scripts = tmp.join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        fs::write(scripts.join("lint.nu"), "# lint").unwrap();
        fs::write(scripts.join("build.nu"), "# build").unwrap();
        fs::write(scripts.join("readme.md"), "not a script").unwrap();

        let cmds = NuScriptSource.discover(&tmp).unwrap();
        assert_eq!(cmds.len(), 2);
        assert!(cmds.iter().any(|c| c.name == "lint"));
        assert!(cmds.iter().any(|c| c.name == "build"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn npm_source_parses_package_json() {
        let tmp = std::env::temp_dir().join("xtui-test-npm-source");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(
            tmp.join("package.json"),
            r#"{"scripts":{"dev":"vite","build":"tsc","test":"jest"}}"#,
        )
        .unwrap();

        let cmds = NpmSource.discover(&tmp).unwrap();
        assert_eq!(cmds.len(), 3);
        assert!(cmds.iter().any(|c| c.name == "dev"));
        assert!(cmds.iter().any(|c| c.name == "build"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn npm_source_empty_without_package_json() {
        let cmds = NpmSource.discover(Path::new("/tmp")).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn make_source_parses_makefile() {
        let tmp = std::env::temp_dir().join("xtui-test-make-source");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(
            tmp.join("Makefile"),
            "build:\n\tcargo build\n\ntest:\n\tcargo test\n\nclean:\n\trm -rf target\n",
        )
        .unwrap();

        let cmds = MakeSource.discover(&tmp).unwrap();
        assert_eq!(cmds.len(), 3);
        assert!(cmds.iter().any(|c| c.name == "build"));
        assert!(cmds.iter().any(|c| c.name == "test"));
        assert!(cmds.iter().any(|c| c.name == "clean"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn make_source_empty_without_makefile() {
        let cmds = MakeSource.discover(Path::new("/tmp")).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn mise_source_parses_tasks() {
        let tmp = std::env::temp_dir().join("xtui-test-mise-source");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(
            tmp.join("mise.toml"),
            r#"
[tasks.lint]
run = "cargo clippy"
description = "Run clippy"

[tasks.fmt]
run = "cargo fmt"
"#,
        )
        .unwrap();

        let cmds = MiseSource.discover(&tmp).unwrap();
        assert_eq!(cmds.len(), 2);
        let lint = cmds.iter().find(|c| c.name == "lint").unwrap();
        assert_eq!(lint.description.as_deref(), Some("Run clippy"));
        let fmt = cmds.iter().find(|c| c.name == "fmt").unwrap();
        assert!(fmt.description.is_none());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn mise_source_empty_without_mise_toml() {
        let cmds = MiseSource.discover(Path::new("/tmp")).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn all_sources_returns_seven() {
        assert_eq!(all_sources().len(), 7);
    }
}
