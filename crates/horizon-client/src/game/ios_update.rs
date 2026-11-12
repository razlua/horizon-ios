//! iOS: whether a newer build is out. The repository's AltStore source (`altstore-source.json`)
//! lists the released builds, newest first; each build carries the workflow's run number as its
//! `CFBundleVersion`, so a higher number there is a newer build than this one.
use std::time::Duration;

/// The source's address.
const SOURCE: &str =
    "https://raw.githubusercontent.com/razlua/horizon-ios/ios-port/altstore-source.json";
/// Where the player gets it: the latest release's page.
pub(super) const RELEASES: &str = "https://github.com/razlua/horizon-ios/releases/latest";

/// The newest released version (`2026.11.12`), when it is newer than the one running. None when
/// this one is the newest, or the check can't be made (offline, say).
pub(super) fn newer() -> Option<String> {
    let installed: u64 = horizon_threads::ios_bundle::info("CFBundleVersion")?.parse().ok()?;
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(10))
        .user_agent(&format!("Horizon/{} (iOS update check)", horizon_core::release::VERSION))
        .build();
    let text = agent.get(SOURCE).call().ok()?.into_string().ok()?;
    let source: serde_json::Value = serde_json::from_str(&text).ok()?;
    let (version, build) = latest(&source)?;
    (build > installed).then_some(version)
}

/// The source's newest version and build number.
fn latest(source: &serde_json::Value) -> Option<(String, u64)> {
    let newest = source.get("apps")?.get(0)?.get("versions")?.get(0)?;
    let version = newest.get("version")?.as_str()?.to_owned();
    let build = newest.get("buildVersion")?.as_str()?.parse().ok()?;
    Some((version, build))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_newest_version_comes_first() {
        let source = serde_json::json!({ "apps": [{ "versions": [
            { "version": "2026.11.12", "buildVersion": "41" },
            { "version": "0.1.0", "buildVersion": "1" }
        ]}]});
        assert_eq!(super::latest(&source), Some(("2026.11.12".into(), 41)));
        assert_eq!(super::latest(&serde_json::json!({})), None);
    }
}
