use std::path::Path;

use anyhow::Result;

use crate::discover::parse_source;

/// A command discovered from a project's xtask or similar source.
#[derive(Debug, Clone)]
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

/// Adapter: discovers commands by parsing `xtask/src/main.rs`.
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
