mod app;
mod ui;

// Re-export lib modules so app.rs/ui.rs can use crate::module paths.
use xtui as xtui_lib;
#[allow(unused_imports)]
pub(crate) use xtui_lib::discover;
pub(crate) use xtui_lib::history;
pub(crate) use xtui_lib::pipeline;
#[allow(unused_imports)]
pub(crate) use xtui_lib::registry;
pub(crate) use xtui_lib::runner;
pub(crate) use xtui_lib::search;
pub(crate) use xtui_lib::source;
pub(crate) use xtui_lib::status;

use std::path::{Path, PathBuf};

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        // Accept any project with a recognizable marker
        let markers = [
            "Cargo.toml",
            "Justfile",
            "justfile",
            "package.json",
            "Makefile",
            "mise.toml",
        ];
        if markers.iter().any(|m| dir.join(m).exists()) {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let workspace = match std::env::args().nth(1) {
        Some(ref flag) if flag == "--path" => {
            let path = std::env::args()
                .nth(2)
                .ok_or_else(|| anyhow::anyhow!("--path requires a directory"))?;
            let dir = PathBuf::from(path).canonicalize()?;
            find_workspace_root(&dir).unwrap_or(dir)
        }
        Some(path) => {
            let dir = PathBuf::from(path).canonicalize()?;
            find_workspace_root(&dir).unwrap_or(dir)
        }
        None => {
            let cwd = std::env::current_dir()?;
            find_workspace_root(&cwd)
                .ok_or_else(|| anyhow::anyhow!("no project found (use --path <dir>)"))?
        }
    };

    let mut app = app::App::new(workspace);
    app.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_workspace_root_finds_self() {
        // xtui now has its own xtask crate, so it should find itself
        let root = std::env::current_dir().unwrap();
        let found = find_workspace_root(&root);
        assert!(found.is_some());
        assert!(found.unwrap().join("xtask").is_dir());
    }

    #[test]
    fn test_find_workspace_root_none_in_tmp() {
        let found = find_workspace_root(Path::new("/tmp"));
        assert!(found.is_none());
    }
}
