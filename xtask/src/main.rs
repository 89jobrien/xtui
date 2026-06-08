use std::env;
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("xtask must be inside workspace")
        .to_path_buf()
}

fn cargo(args: &[&str]) -> ! {
    let root = workspace_root();
    let manifest = root.join("Cargo.toml");
    // Split at "--" so --manifest-path goes before the separator
    let split = args.iter().position(|a| *a == "--");
    let (before, after) = match split {
        Some(i) => (&args[..i], &args[i..]),
        None => (args, [].as_slice()),
    };
    let status = Command::new("cargo")
        .args(before)
        .arg("--manifest-path")
        .arg(&manifest)
        .args(after)
        .status()
        .expect("failed to run cargo");
    std::process::exit(status.code().unwrap_or(1));
}

fn main() {
    let task = env::args().nth(1);
    match task.as_deref() {
        Some("check") => cargo(&["check", "--workspace"]),
        Some("test") => cargo(&["test", "--workspace"]),
        Some("clippy") => cargo(&["clippy", "--workspace", "--", "-D", "warnings"]),
        Some("install") => {
            let root = workspace_root();
            let status = Command::new("cargo")
                .args(["install", "--path"])
                .arg(&root)
                .status()
                .expect("failed to run cargo install");
            std::process::exit(status.code().unwrap_or(1));
        }
        _ => {
            eprintln!("Available commands:");
            eprintln!("    check        Run cargo check");
            eprintln!("    test         Run cargo test");
            eprintln!("    clippy       Run cargo clippy");
            eprintln!("    install      Install xtui to ~/.cargo/bin");
        }
    }
}
