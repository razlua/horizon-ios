//! iOS: whether a newer build is out. The repository's AltStore source (`altstore-source.json`)
//! lists the released builds, newest first; each build carries the workflow's run number as its
//! `CFBundleVersion`, so a higher number there is a newer build than this one.
use std::time::Duration;

/// The source's address.
const SOURCE: &str =
    "https://raw.githubusercontent.com/razlua/horizon-ios/ios-port/altstore-source.json";
/// Where the player gets it: the latest release's page.
pub(super) const RELEASES: &str = "https://github.com/razlua/horizon-ios/releases/latest";

/// A newer build that is out: its version (`2026.11.12`) and its `.ipa`'s address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Newer {
    pub version: String,
    pub ipa: String,
}

/// The apps that install an update in one tap, through their `install` links (AltStore's,
/// which SideStore kept): their name and URL scheme. SideStore first: it refreshes on the phone.
const INSTALLERS: [(&str, &str); 2] = [("SideStore", "sidestore"), ("AltStore", "altstore")];

/// The installed app that can install the update (SideStore or AltStore), if any. Asked once.
pub(super) fn installer() -> Option<&'static str> {
    static FOUND: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
    let found = *FOUND.get_or_init(|| {
        INSTALLERS
            .iter()
            .position(|(_, scheme)| horizon_threads::ios_url::can_open(&format!("{scheme}://")))
    });
    found.map(|i| INSTALLERS[i].0)
}

/// Where the update button goes: `installer`'s install link for the `.ipa`, which signs and
/// installs it over this app (worlds and sign-ins stay), else the releases' page.
pub(super) fn update_link(newer: &Newer) -> String {
    match installer().and_then(|name| INSTALLERS.iter().find(|(n, _)| *n == name)) {
        Some((_, scheme)) => install_link(scheme, &newer.ipa),
        None => RELEASES.to_owned(),
    }
}

/// `<scheme>://install?url=<ipa>`, the address encoded as a query value.
fn install_link(scheme: &str, ipa: &str) -> String {
    let mut encoded = String::new();
    for b in ipa.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char)
            }
            _ => encoded.push_str(&format!("%{b:02X}")),
        }
    }
    format!("{scheme}://install?url={encoded}")
}

/// The newest released build, when it is newer than the one running. None when this one is the
/// newest, or the check can't be made (offline, say).
pub(super) fn newer() -> Option<Newer> {
    let installed: u64 = horizon_threads::ios_bundle::info("CFBundleVersion")?.parse().ok()?;
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(10))
        .user_agent(&format!("Horizon/{} (iOS update check)", horizon_core::release::VERSION))
        .build();
    let text = agent.get(SOURCE).call().ok()?.into_string().ok()?;
    let source: serde_json::Value = serde_json::from_str(&text).ok()?;
    let (newer, build) = latest(&source)?;
    (build > installed).then_some(newer)
}

/// The source's newest build and its build number.
fn latest(source: &serde_json::Value) -> Option<(Newer, u64)> {
    let newest = source.get("apps")?.get(0)?.get("versions")?.get(0)?;
    let version = newest.get("version")?.as_str()?.to_owned();
    let ipa = newest.get("downloadURL")?.as_str()?.to_owned();
    let build = newest.get("buildVersion")?.as_str()?.parse().ok()?;
    Some((Newer { version, ipa }, build))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_newest_version_comes_first() {
        let source = serde_json::json!({ "apps": [{ "versions": [
            { "version": "2026.11.12", "buildVersion": "41", "downloadURL": "https://x/a b.ipa" },
            { "version": "0.1.0", "buildVersion": "1", "downloadURL": "https://x/old.ipa" }
        ]}]});
        let newer = super::Newer { version: "2026.11.12".into(), ipa: "https://x/a b.ipa".into() };
        assert_eq!(super::latest(&source), Some((newer, 41)));
        assert_eq!(super::latest(&serde_json::json!({})), None);
    }

    #[test]
    fn the_install_link_carries_the_ipa_encoded() {
        assert_eq!(
            super::install_link("sidestore", "https://x/a b.ipa?v=1"),
            "sidestore://install?url=https%3A%2F%2Fx%2Fa%20b.ipa%3Fv%3D1"
        );
    }
}
