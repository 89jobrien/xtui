use std::{env, path::PathBuf, process::Command};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();

    match env::args().nth(1).as_deref() {
        Some("xstate") => {
            let verify = env::args().any(|a| a == "--verify");
            let mut cmd = Command::new("nu");
            cmd.arg(root.join("scripts/xstate.nu")).current_dir(&root);
            if verify {
                cmd.arg("--verify");
            }
            let status = cmd.status().expect("nu not found — is nushell installed?");
            std::process::exit(status.code().unwrap_or(1));
        }
        other => {
            eprintln!("fixture xtask: unknown command {:?}", other);
            eprintln!("usage: xtask xstate [--verify]");
            std::process::exit(1);
        }
    }
}
