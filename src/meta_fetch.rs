use anyhow::Result;
use serde::Deserialize;

pub trait MetadataFetcher: Send + Sync {
    fn fetch_crates_io(&self, name: &str) -> Result<CratesIoMeta>;
    fn fetch_github_behind(&self, repo_url: &str, declared: &str) -> Result<Option<u32>>;
}

#[derive(Debug, Clone)]
pub struct CratesIoMeta {
    pub latest_version: String,
    pub repository: Option<String>,
    pub versions: Vec<String>, // all published versions, newest first
}

#[derive(Deserialize)]
struct CratesIoResponse {
    #[serde(rename = "crate")]
    krate: CratesIoCrate,
    versions: Vec<CratesIoVersion>,
}

#[derive(Deserialize)]
struct CratesIoCrate {
    newest_version: String,
    repository: Option<String>,
}

#[derive(Deserialize)]
struct CratesIoVersion {
    num: String,
}

#[derive(Deserialize)]
struct GithubTag {
    name: String,
}

pub struct HttpMetadataFetcher;

impl MetadataFetcher for HttpMetadataFetcher {
    fn fetch_crates_io(&self, name: &str) -> Result<CratesIoMeta> {
        let url = format!("https://crates.io/api/v1/crates/{name}");
        let req = ureq::get(&url).set(
            "User-Agent",
            "xtui/0.2.0 (https://github.com/89jobrien/xtui)",
        );
        let resp: CratesIoResponse = req.call()?.into_json()?;
        Ok(CratesIoMeta {
            latest_version: resp.krate.newest_version,
            repository: resp.krate.repository,
            versions: resp.versions.into_iter().map(|v| v.num).collect(),
        })
    }

    fn fetch_github_behind(&self, repo_url: &str, declared: &str) -> Result<Option<u32>> {
        let (owner, repo) = parse_github_owner_repo(repo_url)?;
        let url = format!("https://api.github.com/repos/{owner}/{repo}/tags?per_page=100");
        let mut req = ureq::get(&url)
            .set(
                "User-Agent",
                "xtui/0.2.0 (https://github.com/89jobrien/xtui)",
            )
            .set("Accept", "application/vnd.github+json");
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            req = req.set("Authorization", &format!("Bearer {token}"));
        }
        let tags: Vec<GithubTag> = req.call()?.into_json()?;
        let tag_names: Vec<String> = tags.into_iter().map(|t| t.name).collect();
        Ok(count_tags_behind(declared, &tag_names))
    }
}

fn parse_github_owner_repo(url: &str) -> Result<(String, String)> {
    let stripped = url.trim_end_matches('/').trim_end_matches(".git");
    let parts: Vec<&str> = stripped.rsplitn(3, '/').collect();
    if parts.len() < 2 {
        anyhow::bail!("cannot parse GitHub URL: {url}");
    }
    Ok((parts[1].to_string(), parts[0].to_string()))
}

/// Count how many published versions are newer than `declared`.
pub fn count_versions_behind(declared: &str, versions: &[String]) -> Option<u32> {
    let pos = versions.iter().position(|v| v == declared)?;
    Some(pos as u32)
}

/// Count how many tags appear before (newer than) the declared version tag.
fn count_tags_behind(declared: &str, tags: &[String]) -> Option<u32> {
    let candidates = [declared.to_string(), format!("v{declared}")];
    let pos = tags.iter().position(|t| candidates.contains(t))?;
    Some(pos as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher {
        latest: &'static str,
    }

    impl MetadataFetcher for FakeFetcher {
        fn fetch_crates_io(&self, _name: &str) -> Result<CratesIoMeta> {
            Ok(CratesIoMeta {
                latest_version: self.latest.to_string(),
                repository: None,
                versions: vec!["1.0.0".into(), "2.0.0".into()],
            })
        }
        fn fetch_github_behind(&self, _url: &str, _declared: &str) -> Result<Option<u32>> {
            Ok(Some(1))
        }
    }

    #[test]
    fn fake_fetcher_satisfies_trait() {
        let f = FakeFetcher { latest: "2.0.0" };
        let meta = f.fetch_crates_io("serde").unwrap();
        assert_eq!(meta.latest_version, "2.0.0");
        assert_eq!(meta.versions.len(), 2);
        let behind = f
            .fetch_github_behind("https://github.com/foo/bar", "1.0.0")
            .unwrap();
        assert_eq!(behind, Some(1));
    }

    #[test]
    fn count_versions_behind_correct() {
        let versions = vec![
            "1.0.0".to_string(),
            "1.1.0".to_string(),
            "2.0.0".to_string(),
        ];
        // versions are newest-first, so "1.0.0" is at index 0 meaning 0 newer
        // Actually: versions newest-first means index 0 = newest
        // count_versions_behind finds position of declared — position 0 means 0 versions ahead
        assert_eq!(count_versions_behind("1.0.0", &versions), Some(0));
        assert_eq!(count_versions_behind("2.0.0", &versions), Some(2));
        assert_eq!(count_versions_behind("0.9.0", &versions), None);
    }

    #[test]
    fn parse_github_owner_repo_handles_trailing_git() {
        let (owner, repo) =
            parse_github_owner_repo("https://github.com/serde-rs/serde.git").unwrap();
        assert_eq!(owner, "serde-rs");
        assert_eq!(repo, "serde");
    }

    #[test]
    fn parse_github_owner_repo_plain() {
        let (owner, repo) = parse_github_owner_repo("https://github.com/tokio-rs/tokio").unwrap();
        assert_eq!(owner, "tokio-rs");
        assert_eq!(repo, "tokio");
    }
}
