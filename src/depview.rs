use std::path::Path;

#[derive(Debug, Clone)]
pub struct DepInfo {
    pub name: String,
    pub declared_version: String,
    pub crates_io_latest: Option<String>,
    pub github_url: Option<String>,
    pub versions_behind: Option<u32>,
    pub state: DepFetchState,
}

#[derive(Debug, Clone)]
pub enum DepFetchState {
    Loading,
    Ready,
    Error(String),
}

/// Returns direct dependencies of all workspace members as Loading stubs.
/// Returns empty vec if project has no Cargo.toml or krates fails.
pub fn collect_direct_deps(project: &Path) -> Vec<DepInfo> {
    let cargo_toml = project.join("Cargo.toml");
    if !cargo_toml.exists() {
        return vec![];
    }
    let mut cmd = krates::Cmd::new();
    cmd.manifest_path(&cargo_toml);
    // Exclude dev and build deps — show only production (normal) dependencies.
    // ignore_kind returns &mut Self so we must mutate then call build separately.
    let mut builder = krates::Builder::new();
    builder.ignore_kind(krates::DepKind::Dev, krates::Scope::All);
    builder.ignore_kind(krates::DepKind::Build, krates::Scope::All);
    let Ok(graph) = builder.build(cmd, |_: krates::cm::Package| {}) else {
        return vec![];
    };
    let graph: krates::Krates<krates::cm::Package> = graph;

    let mut seen = std::collections::HashSet::new();
    let mut deps = Vec::new();

    // Collect NodeIds for all workspace members
    let member_ids: Vec<krates::NodeId> = graph
        .workspace_members()
        .filter_map(|node| {
            if let krates::Node::Krate { id, .. } = node {
                graph.nid_for_kid(id)
            } else {
                None
            }
        })
        .collect();

    for nid in member_ids {
        for direct in graph.direct_dependencies(nid) {
            let dep = direct.krate;
            let key = format!("{}@{}", dep.name, dep.version);
            if seen.insert(key) {
                deps.push(DepInfo {
                    name: dep.name.clone(),
                    declared_version: dep.version.to_string(),
                    crates_io_latest: None,
                    github_url: None,
                    versions_behind: None,
                    state: DepFetchState::Loading,
                });
            }
        }
    }
    deps.sort_by(|a, b| a.name.cmp(&b.name));
    deps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dep_info_default_state_is_loading() {
        let d = DepInfo {
            name: "foo".into(),
            declared_version: "1.0.0".into(),
            crates_io_latest: None,
            github_url: None,
            versions_behind: None,
            state: DepFetchState::Loading,
        };
        assert!(matches!(d.state, DepFetchState::Loading));
    }

    #[test]
    fn collect_direct_deps_empty_for_nonexistent_path() {
        let result = collect_direct_deps(Path::new("/nonexistent/path"));
        assert!(result.is_empty());
    }
}
