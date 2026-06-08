use anyhow::Result;
use regex::Regex;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct XtaskCommand {
    pub name: String,
    pub description: Option<String>,
}

pub fn parse_source(source: &str) -> Vec<XtaskCommand> {
    // Extract command names from match arms like Some("name")
    let re = Regex::new(r#"Some\("([^"]+)"\)"#).unwrap();
    let names: Vec<String> = re
        .captures_iter(source)
        .map(|caps| caps[1].to_string())
        .collect();

    // Try to find descriptions from help/usage print lines like:
    //   eprintln!("    name         Description text");
    let help_re =
        Regex::new(r#"(?:println!|eprintln!)\(\s*"\\?\s{2,}(\S+)\s{2,}(.+?)\\?"\s*\)"#).unwrap();
    let mut desc_map = std::collections::HashMap::new();
    for caps in help_re.captures_iter(source) {
        desc_map.insert(caps[1].to_string(), caps[2].trim().to_string());
    }

    names
        .into_iter()
        .map(|name| {
            let description = desc_map.get(&name).cloned();
            XtaskCommand { name, description }
        })
        .collect()
}

pub async fn discover_commands(workspace: &Path) -> Result<Vec<XtaskCommand>> {
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

    #[test]
    fn test_parse_source_with_descriptions() {
        let source = r#"
            match task.as_deref() {
                Some("check") => cargo(&["check"]),
                Some("test") => cargo(&["test"]),
                _ => {
                    eprintln!("    check        Run cargo check");
                    eprintln!("    test         Run cargo test");
                }
            }
        "#;
        let cmds = parse_source(source);
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].name, "check");
        assert_eq!(cmds[0].description, Some("Run cargo check".to_string()));
        assert_eq!(cmds[1].name, "test");
        assert_eq!(cmds[1].description, Some("Run cargo test".to_string()));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn parse_source_never_panics(input in any::<String>()) {
            let _ = parse_source(&input);
        }

        #[test]
        fn parsed_commands_have_nonempty_names(input in any::<String>()) {
            let cmds = parse_source(&input);
            for cmd in &cmds {
                prop_assert!(!cmd.name.is_empty());
            }
        }

        #[test]
        fn parse_source_is_idempotent(input in any::<String>()) {
            let first = parse_source(&input);
            let second = parse_source(&input);
            prop_assert_eq!(first.len(), second.len());
            for (a, b) in first.iter().zip(second.iter()) {
                prop_assert_eq!(&a.name, &b.name);
                prop_assert_eq!(&a.description, &b.description);
            }
        }
    }
}
