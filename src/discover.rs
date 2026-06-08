use anyhow::Result;
use regex::Regex;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct XtaskCommand {
    pub name: String,
    pub description: Option<String>,
}

pub fn parse_help_output(output: &str) -> Vec<XtaskCommand> {
    let re = Regex::new(r"^\s{2,}(\S+)\s+(.+)$").unwrap();
    let re_bare = Regex::new(r"^\s{2,}(\S+)\s*$").unwrap();
    let mut cmds = Vec::new();
    for line in output.lines() {
        if let Some(caps) = re.captures(line) {
            cmds.push(XtaskCommand {
                name: caps[1].to_string(),
                description: Some(caps[2].trim().to_string()),
            });
        } else if let Some(caps) = re_bare.captures(line) {
            cmds.push(XtaskCommand {
                name: caps[1].to_string(),
                description: None,
            });
        }
    }
    cmds
}

pub fn parse_source(source: &str) -> Vec<XtaskCommand> {
    let re = Regex::new(r#"Some\("([^"]+)"\)"#).unwrap();
    re.captures_iter(source)
        .map(|caps| XtaskCommand {
            name: caps[1].to_string(),
            description: None,
        })
        .collect()
}

pub async fn discover_commands(workspace: &Path) -> Result<Vec<XtaskCommand>> {
    let xtask_manifest = workspace.join("xtask/Cargo.toml");
    if !xtask_manifest.exists() {
        return Ok(vec![]);
    }

    let output = tokio::process::Command::new("cargo")
        .args(["run", "--quiet", "--manifest-path"])
        .arg(&xtask_manifest)
        .arg("--")
        .output()
        .await?;

    let text = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{text}{stderr}");

    let cmds = parse_help_output(&combined);
    if !cmds.is_empty() {
        return Ok(cmds);
    }

    // Fallback: parse source
    let main_path = workspace.join("xtask/src/main.rs");
    if main_path.exists() {
        let source = tokio::fs::read_to_string(&main_path).await?;
        let cmds = parse_source(&source);
        if !cmds.is_empty() {
            return Ok(cmds);
        }
    }

    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_help_lines() {
        let output = "\
Available commands:
    verify       Run all checks
    lint         Run clippy
    fix          Auto-fix warnings";
        let cmds = parse_help_output(output);
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0].name, "verify");
        assert_eq!(cmds[0].description, Some("Run all checks".to_string()));
    }

    #[test]
    fn test_parse_help_lines_no_description() {
        let output = "    verify\n    lint\n";
        let cmds = parse_help_output(output);
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].description, None);
    }

    #[test]
    fn test_parse_source_fallback() {
        let source = r#"
            match cmd.as_deref() {
                Some("verify") => verify(),
                Some("lint") => lint(),
                _ => usage(),
            }
        "#;
        let cmds = parse_source(source);
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].name, "verify");
        assert_eq!(cmds[0].description, None);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn parse_help_never_panics(input in any::<String>()) {
            let _ = parse_help_output(&input);
        }

        #[test]
        fn parse_source_never_panics(input in any::<String>()) {
            let _ = parse_source(&input);
        }

        #[test]
        fn parsed_commands_have_nonempty_names(input in any::<String>()) {
            let cmds = parse_help_output(&input);
            for cmd in &cmds {
                prop_assert!(!cmd.name.is_empty());
            }
            let cmds = parse_source(&input);
            for cmd in &cmds {
                prop_assert!(!cmd.name.is_empty());
            }
        }

        #[test]
        fn parse_help_is_idempotent(input in any::<String>()) {
            let first = parse_help_output(&input);
            let second = parse_help_output(&input);
            prop_assert_eq!(first.len(), second.len());
            for (a, b) in first.iter().zip(second.iter()) {
                prop_assert_eq!(&a.name, &b.name);
                prop_assert_eq!(&a.description, &b.description);
            }
        }
    }
}
