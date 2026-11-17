//! iOS and Android: whether a newer build is out. The repository's AltStore source
//! (`altstore-source.json`) lists the released builds, newest first; each build carries the
//! workflow's run number as its `CFBundleVersion`, so a higher number there is a newer build than
//! this one. Android's builds are listed the same way in a file of their own.
use std::time::Duration;

/// The source's address.
const SOURCE: &str = if cfg!(target_os = "android") {
    "https://raw.githubusercontent.com/razlua/horizon-ios/android-port/android-source.json"
} else {
    "https://raw.githubusercontent.com/razlua/horizon-ios/ios-port/altstore-source.json"
};
/// Where the player gets it: the releases' page.
pub(super) const RELEASES: &str = if cfg!(target_os = "android") {
    "https://github.com/razlua/horizon-ios/releases"
} else {
    "https://github.com/razlua/horizon-ios/releases/latest"
};

/// A newer build that is out: its version (`2026.11.12`) and its `.ipa`'s (or `.apk`'s) address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Newer {
    pub version: String,
    pub ipa: String,
}

/// The apps that install an update in one tap, through their `install` links (AltStore's,
/// which SideStore kept): their name and URL scheme. SideStore first: it refreshes on the phone.
#[cfg(target_os = "ios")]
const INSTALLERS: [(&str, &str); 2] = [("SideStore", "sidestore"), ("AltStore", "altstore")];

/// The installed app that can install the update (SideStore or AltStore), if any. Asked once.
/// Android has none: the `.apk` is opened.
pub(super) fn installer() -> Option<&'static str> {
    #[cfg(target_os = "ios")]
    {
        static FOUND: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
        let found = *FOUND.get_or_init(|| {
            INSTALLERS
                .iter()
                .position(|(_, scheme)| horizon_threads::ios_url::can_open(&format!("{scheme}://")))
        });
        found.map(|i| INSTALLERS[i].0)
    }
    #[cfg(not(target_os = "ios"))]
    {
        None
    }
}

/// Where the update button goes. iOS: `installer`'s install link for the `.ipa`, which signs and
/// installs it over this app (worlds and sign-ins stay), else the releases' page. Android: the
/// `.apk` itself, which installs over this one.
pub(super) fn update_link(newer: &Newer) -> String {
    #[cfg(target_os = "ios")]
    if let Some((_, scheme)) = installer().and_then(|name| INSTALLERS.iter().find(|(n, _)| *n == name)) {
        return install_link(scheme, &newer.ipa);
    }
    if cfg!(target_os = "android") && !newer.ipa.is_empty() {
        return newer.ipa.clone();
    }
    RELEASES.to_owned()
}

/// `<scheme>://install?url=<ipa>`, the address encoded as a query value.
#[cfg_attr(not(any(target_os = "ios", test)), allow(dead_code))]
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

/// This build's number: the workflow's run number.
fn installed() -> Option<u64> {
    #[cfg(target_os = "ios")]
    {
        horizon_threads::ios_bundle::info("CFBundleVersion")?.parse().ok()
    }
    #[cfg(not(target_os = "ios"))]
    {
        option_env!("HORIZON_ANDROID_BUILD")?.parse().ok()
    }
}

/// The newest released build, when it is newer than the one running. None when this one is the
/// newest, or the check can't be made (offline, say).
pub(super) fn newer() -> Option<Newer> {
    let installed = installed()?;
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
    let ipa = newest
        .get("downloadURL")
        .and_then(|u| u.as_str())
        .unwrap_or_default()
        .to_owned();
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
