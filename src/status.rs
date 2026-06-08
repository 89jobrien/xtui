use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitStatus {
    pub branch: String,
    pub dirty: bool,
    pub ahead: u32,
    pub behind: u32,
    pub recent_commits: Vec<String>,
    pub diff_stat: String,
}

fn run(project: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(project)
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}

pub fn collect_git_status(project: &Path) -> Option<GitStatus> {
    // Fails if not a git repo — used as the early-out sentinel
    let branch_raw = run(project, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let branch = branch_raw.trim().to_owned();

    let dirty = run(project, &["status", "--porcelain"])
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    let (ahead, behind) = run(
        project,
        &["rev-list", "--left-right", "--count", "@{u}...HEAD"],
    )
    .and_then(|s| {
        let mut parts = s.split_whitespace();
        let b: u32 = parts.next()?.parse().ok()?;
        let a: u32 = parts.next()?.parse().ok()?;
        Some((a, b))
    })
    .unwrap_or((0, 0));

    let recent_commits = run(project, &["log", "--oneline", "-5"])
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();

    let diff_stat = run(project, &["diff", "--stat"])
        .unwrap_or_default()
        .trim()
        .to_owned();

    Some(GitStatus {
        branch,
        dirty,
        ahead,
        behind,
        recent_commits,
        diff_stat,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_git_status_on_self() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let status = collect_git_status(root);
        assert!(status.is_some());
        let s = status.unwrap();
        assert!(!s.branch.is_empty());
        assert!(!s.recent_commits.is_empty());
    }

    #[test]
    fn test_collect_git_status_non_repo() {
        let status = collect_git_status(Path::new("/tmp"));
        assert!(status.is_none());
    }
}
