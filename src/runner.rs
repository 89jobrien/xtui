use anyhow::Result;
use futures_util::StreamExt;
use std::path::Path;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, LinesCodec};

/// Encapsulates a running child process and its line-output channel.
pub struct RunningTask {
    child: Child,
    rx: mpsc::Receiver<String>,
}

impl Drop for RunningTask {
    fn drop(&mut self) {
        // Best-effort sync kill to avoid orphaned processes on quit
        let _ = self.child.start_kill();
    }
}

impl RunningTask {
    /// Drain any buffered lines from the output channel.
    pub fn poll_lines(&mut self, buf: &mut Vec<String>) {
        while let Ok(line) = self.rx.try_recv() {
            buf.push(line);
        }
    }

    /// Check if the child has exited; return exit code if so.
    pub fn try_exit_code(&mut self) -> Option<i32> {
        match self.child.try_wait() {
            Ok(Some(status)) => status.code(),
            _ => None,
        }
    }

    /// Kill the child process.
    pub async fn cancel(&mut self) {
        let _ = self.child.kill().await;
    }
}

pub async fn spawn_command(program: &str, args: &[&str], cwd: &Path) -> Result<RunningTask> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let (tx, rx) = mpsc::channel(1024);
    let tx2 = tx.clone();

    tokio::spawn(async move {
        let mut reader = FramedRead::new(stdout, LinesCodec::new());
        while let Some(Ok(line)) = reader.next().await {
            if tx.send(line).await.is_err() {
                break;
            }
        }
    });

    tokio::spawn(async move {
        let mut reader = FramedRead::new(stderr, LinesCodec::new());
        while let Some(Ok(line)) = reader.next().await {
            if tx2.send(line).await.is_err() {
                break;
            }
        }
    });

    Ok(RunningTask { child, rx })
}

pub async fn run_xtask(workspace: &Path, command: &str) -> Result<RunningTask> {
    let manifest = workspace.join("xtask/Cargo.toml");
    let manifest_str = manifest.to_string_lossy().to_string();
    let parts: Vec<&str> = command.split_whitespace().collect();
    let mut args = vec!["run", "--quiet", "--manifest-path", &manifest_str, "--"];
    args.extend(parts);
    spawn_command("cargo", &args, workspace).await
}

pub async fn run_source_command(
    workspace: &Path,
    cmd: &crate::source::SourceCommand,
) -> Result<RunningTask> {
    match cmd.source.as_str() {
        "xtask" => run_xtask(workspace, &cmd.name).await,
        "cargo" => {
            let parts: Vec<&str> = cmd.name.split_whitespace().collect();
            spawn_command("cargo", &parts, workspace).await
        }
        "just" => spawn_command("just", &[cmd.name.as_str()], workspace).await,
        "nu" => {
            let script = format!("scripts/{}.nu", cmd.name);
            spawn_command("nu", &[script.as_str()], workspace).await
        }
        "npm" => spawn_command("npm", &["run", &cmd.name], workspace).await,
        "make" => spawn_command("make", &[cmd.name.as_str()], workspace).await,
        "mise" => spawn_command("mise", &["run", &cmd.name], workspace).await,
        "cargo-bin" => {
            let parts: Vec<&str> = cmd.name.split_whitespace().collect();
            let (bin, args) = parts.split_first().unwrap_or((&"", &[]));
            let bin_path = dirs::home_dir()
                .ok_or_else(|| anyhow::anyhow!("no home directory"))?
                .join(".cargo")
                .join("bin")
                .join(bin);
            spawn_command(&bin_path.to_string_lossy(), args, workspace).await
        }
        other => anyhow::bail!("unknown source: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_captures_output() {
        let mut task = spawn_command("echo", &["hello"], std::path::Path::new("."))
            .await
            .unwrap();

        // Wait for process to finish
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let mut lines = Vec::new();
        task.poll_lines(&mut lines);
        assert!(
            lines.iter().any(|l| l.contains("hello")),
            "expected 'hello' in output, got: {lines:?}"
        );
    }
}
