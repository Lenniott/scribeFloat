use crate::types::UpdateCheckResult;
use reqwest::header;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASES_URL: &str = "https://api.github.com/repos/Lenniott/scribefloat/releases/latest";
const REPO_RELEASES_BASE: &str = "https://github.com/Lenniott/scribefloat/releases/tag/";

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
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
            release_url: release_url_for_tag(&release.tag_name),
            release_notes,
        })
    }
}

/// Builds a release URL from our own trusted repo constant rather than the
/// API-supplied `html_url`, so a compromised/MITM'd release response can't
/// redirect users to an arbitrary URL or scheme when they click "Open download page".
fn release_url_for_tag(tag_name: &str) -> String {
    let safe_tag = tag_name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect::<String>();
    format!("{REPO_RELEASES_BASE}{safe_tag}")
}

fn is_newer(candidate: &str, current: &str) -> bool {
    parse_version(candidate) > parse_version(current)
}

fn parse_version(v: &str) -> (u32, u32, u32) {
    // Take only the leading digits of each component so pre-release/build suffixes
    // (e.g. "1.2.3-beta", "1.2.3+build") don't collapse the component to 0.
    let mut parts = v.splitn(3, '.').map(|p| {
        let digits: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse::<u32>().unwrap_or(0)
    });
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}
