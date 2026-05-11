use crate::types::UpdateCheckResult;
use reqwest::header;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASES_URL: &str =
    "https://api.github.com/repos/Lenniott/liscribe_v8/releases/latest";

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
}

pub struct UpdateService;

impl UpdateService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    pub async fn check_for_update(&self) -> Result<UpdateCheckResult, String> {
        let client = reqwest::Client::builder()
            .user_agent(format!("scribefloat/{CURRENT_VERSION}"))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| format!("failed to build HTTP client: {e}"))?;

        let response = client
            .get(RELEASES_URL)
            .header(header::ACCEPT, "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| format!("could not reach update server: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("update server returned {}", response.status()));
        }

        let release: GithubRelease = response
            .json()
            .await
            .map_err(|e| format!("unexpected update response: {e}"))?;

        let latest_version = release.tag_name.trim_start_matches('v').to_string();
        let update_available = is_newer(&latest_version, CURRENT_VERSION);

        let release_notes: String = release
            .body
            .unwrap_or_default()
            .split("\n\n")
            .next()
            .unwrap_or("")
            .trim()
            .chars()
            .take(500)
            .collect();

        Ok(UpdateCheckResult {
            update_available,
            latest_version,
            current_version: CURRENT_VERSION.to_string(),
            release_url: release.html_url,
            release_notes,
        })
    }
}

fn is_newer(candidate: &str, current: &str) -> bool {
    parse_version(candidate) > parse_version(current)
}

fn parse_version(v: &str) -> (u32, u32, u32) {
    let mut parts = v.splitn(3, '.').map(|p| p.parse::<u32>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}
