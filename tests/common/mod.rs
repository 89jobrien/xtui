use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct ProjectFixture {
    root: PathBuf,
}

impl ProjectFixture {
    pub fn new() -> Self {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let root = std::env::temp_dir().join(format!("xtui-fixture-{pid}-{id}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn with_cargo_toml(self, content: &str) -> Self {
        fs::write(self.root.join("Cargo.toml"), content).unwrap();
        self
    }

    pub fn with_justfile(self, content: &str) -> Self {
        fs::write(self.root.join("Justfile"), content).unwrap();
        self
    }

    pub fn with_package_json(self, content: &str) -> Self {
        fs::write(self.root.join("package.json"), content).unwrap();
        self
    }

    pub fn with_makefile(self, content: &str) -> Self {
        fs::write(self.root.join("Makefile"), content).unwrap();
        self
    }

    pub fn with_mise_toml(self, content: &str) -> Self {
        fs::write(self.root.join("mise.toml"), content).unwrap();
        self
    }

    pub fn with_nu_script(self, name: &str, content: &str) -> Self {
        let dir = self.root.join("scripts");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("{name}.nu")), content).unwrap();
        self
    }

    pub fn with_xtask_main(self, content: &str) -> Self {
        let dir = self.root.join("xtask/src");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("main.rs"), content).unwrap();
        self
    }
}

impl Drop for ProjectFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
