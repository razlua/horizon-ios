//! The on-device setup (iOS): the game's prepared resources without a computer. Horizon's
//! preparation runs Mojang's code on Java for a few tables, which a phone can't; those tables are
//! published once (the repository's `tables-<revision>` release: no file of Mojang's in it), and
//! the rest is the player's own Minecraft, downloaded from Mojang here after a Microsoft sign-in
//! that owns Java Edition: the client jar's assets and data, and the asset index's objects.
//!
//! The result is the folder a computer's preparation makes, file for file. Downloads resume: a
//! file already in place with Mojang's hash is kept.
#![cfg_attr(not(any(target_os = "ios", target_os = "android")), allow(dead_code))]

use sha1::Digest as _;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::time::Duration;

/// The tables for this build's revision of Horizon and its Minecraft.
pub const TABLES_URL: &str = "https://github.com/razlua/horizon-ios/releases/download/tables-6475f62/horizon-tables-26.3-6475f62.zip";
pub const TABLES_SHA256: &str = "8ccdbd7725e78952379e0a792d9ce851e058b192abe97aafacca3b64684e8862";
/// Mojang's asset objects, by hash.
const OBJECTS: &str = "https://resources.download.minecraft.net";
/// Downloads at once.
const LANES: usize = 8;
const TRIES: usize = 4;

/// How far the setup is, for the start screen.
#[derive(Default)]
pub struct Progress {
    pub done: AtomicU64,
    pub total: AtomicU64,
    pub stage: Mutex<String>,
    /// The player left: stop at the next file.
    pub cancel: AtomicBool,
}

impl Progress {
    fn stage(&self, text: &str) {
        log::info!("Horizon setup: {text}");
        *self.stage.lock().unwrap_or_else(|e| e.into_inner()) = text.to_owned();
    }
}

/// Whether `resources` holds a finished preparation.
pub fn prepared(resources: &Path) -> bool {
    resources.join("horizon-resources.json").is_file()
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        .user_agent(&format!("Horizon/{} (iOS setup)", horizon_core::release::VERSION))
        .build()
}

/// `url` into `to`, checked with `check` (a hex digest of the bytes); the bytes count in
/// `progress` as they come. A file already there with the right digest is kept.
fn download(
    agent: &ureq::Agent,
    url: &str,
    to: &Path,
    check: &dyn Fn(&[u8]) -> String,
    expected: &str,
    progress: &Progress,
) -> Result<(), String> {
    if let Ok(bytes) = std::fs::read(to)
        && check(&bytes) == expected
    {
        progress.done.fetch_add(bytes.len() as u64, Relaxed);
        return Ok(());
    }
    let mut last = String::new();
    for attempt in 0..TRIES {
        if progress.cancel.load(Relaxed) {
            return Err("cancelled".into());
        }
        if attempt > 0 {
            std::thread::sleep(Duration::from_secs(1 << attempt));
        }
        let mut counted = 0u64;
        let result = (|| -> Result<Vec<u8>, String> {
            let response = agent.get(url).call().map_err(|e| e.to_string())?;
            let mut reader = response.into_reader();
            let mut bytes = Vec::new();
            let mut chunk = [0u8; 64 * 1024];
            loop {
                let n = reader.read(&mut chunk).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&chunk[..n]);
                counted += n as u64;
                progress.done.fetch_add(n as u64, Relaxed);
            }
            Ok(bytes)
        })();
        match result {
            Ok(bytes) if check(&bytes) == expected => {
                if let Some(parent) = to.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                return std::fs::write(to, &bytes).map_err(|e| format!("{}: {e}", to.display()));
            }
            Ok(_) => last = format!("{url}: the download does not match its checksum"),
            Err(e) => last = format!("{url}: {e}"),
        }
        // What this try counted comes off again: the next try counts the file anew.
        progress.done.fetch_sub(counted, Relaxed);
    }
    Err(last)
}

fn hex_sha1(bytes: &[u8]) -> String {
    hex(&sha1::Sha1::digest(bytes))
}

fn hex_sha256(bytes: &[u8]) -> String {
    hex(&sha2::Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn text(v: &serde_json::Value, key: &str) -> Result<String, String> {
    v[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("ios-setup.json: no {key}"))
}

/// A relative path from the setup's lists, refused if it would leave the folder.
fn inside(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let path = Path::new(rel);
    if path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(format!("ios-setup.json: a path outside the folder: {rel}"));
    }
    Ok(root.join(path))
}

/// Prepares `resources` (Documents/resources): the tables, then the player's own Minecraft from
/// Mojang. `work` keeps the downloads until the end, so a setup cut short goes on from there.
pub fn run(resources: &Path, work: &Path, progress: &Progress) -> Result<(), String> {
    let agent = agent();
    std::fs::create_dir_all(work).map_err(|e| format!("{}: {e}", work.display()))?;
    let staging = work.join("resources");

    // The tables, checked against this build's own record of them.
    progress.stage("Downloading Horizon's tables");
    let tables = work.join("tables.zip");
    download(&agent, TABLES_URL, &tables, &hex_sha256, TABLES_SHA256, progress)?;
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&tables).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let setup: serde_json::Value = {
        let mut text = String::new();
        zip.by_name("ios-setup.json")
            .map_err(|e| e.to_string())?
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| e.to_string())?
    };
    let client = &setup["client"];
    let index = &setup["asset_index"];
    let objects = setup["objects"].as_array().cloned().unwrap_or_default();
    let jar_files: Vec<String> = setup["jar_files"]
        .as_array()
        .map(|l| l.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
        .unwrap_or_default();
    // What is left to download, for the progress bar (the tables are counted already).
    let rest = client["size"].as_u64().unwrap_or(0)
        + index["size"].as_u64().unwrap_or(0)
        + objects.iter().filter_map(|o| o["size"].as_u64()).sum::<u64>();
    progress.total.store(progress.done.load(Relaxed) + rest, Relaxed);

    progress.stage("Unpacking Horizon's tables");
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().map_err(|e| e.to_string())?.into_owned();
        let Some(rel) = name.strip_prefix("tables/").map(str::to_owned) else {
            continue;
        };
        if file.is_dir() {
            continue;
        }
        let path = inside(&staging, &rel)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        std::io::copy(&mut file, &mut out).map_err(|e| e.to_string())?;
    }

    // The client jar: its assets and data, as a computer's preparation copies them.
    progress.stage("Downloading Minecraft 26.3 from Mojang");
    let jar = work.join("client.jar");
    download(&agent, &text(client, "url")?, &jar, &hex_sha1, &text(client, "sha1")?, progress)?;
    progress.stage("Unpacking Minecraft's files");
    let mut client_zip = zip::ZipArchive::new(std::io::BufReader::new(
        std::fs::File::open(&jar).map_err(|e| e.to_string())?,
    ))
    .map_err(|e| e.to_string())?;
    for rel in &jar_files {
        if progress.cancel.load(Relaxed) {
            return Err("cancelled".into());
        }
        let path = inside(&staging, rel)?;
        let mut entry = client_zip
            .by_name(rel)
            .map_err(|e| format!("the client jar has no {rel}: {e}"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        std::fs::write(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    }

    // The asset index and its objects: sounds, languages, fonts and the rest.
    progress.stage("Downloading Minecraft's sounds and languages");
    download(
        &agent,
        &text(index, "url")?,
        &staging.join("asset-index.json"),
        &hex_sha1,
        &text(index, "sha1")?,
        progress,
    )?;
    let next = AtomicUsize::new(0);
    let failed: Mutex<Option<String>> = Mutex::new(None);
    std::thread::scope(|scope| {
        for _ in 0..LANES {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Relaxed);
                    let Some(object) = objects.get(i) else {
                        return;
                    };
                    if failed.lock().unwrap_or_else(|e| e.into_inner()).is_some() {
                        return;
                    }
                    let result = (|| -> Result<(), String> {
                        let name = object["name"].as_str().ok_or("an object with no name")?;
                        let hash = object["hash"].as_str().ok_or("an object with no hash")?;
                        if hash.len() < 2 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
                            return Err(format!("a bad object hash: {hash}"));
                        }
                        let path = inside(&staging.join("assets"), name)?;
                        let url = format!("{OBJECTS}/{}/{hash}", &hash[..2]);
                        download(&agent, &url, &path, &hex_sha1, hash, progress)
                    })();
                    if let Err(e) = result {
                        *failed.lock().unwrap_or_else(|e| e.into_inner()) = Some(e);
                        return;
                    }
                }
            });
        }
    });
    if let Some(e) = failed.into_inner().unwrap_or_else(|e| e.into_inner()) {
        return Err(e);
    }

    // The finished folder takes the place of any earlier one.
    progress.stage("Finishing");
    if !prepared(&staging) {
        return Err("the tables have no horizon-resources.json".into());
    }
    std::fs::write(staging.join(".setup-from"), TABLES_SHA256).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(resources);
    std::fs::rename(&staging, resources).map_err(|e| format!("{}: {e}", resources.display()))?;
    let _ = std::fs::remove_dir_all(work);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_from_the_lists_stay_in_the_folder() {
        let root = Path::new("/r");
        assert_eq!(inside(root, "assets/a.png").unwrap(), root.join("assets/a.png"));
        assert!(inside(root, "../x").is_err());
        assert!(inside(root, "/etc/x").is_err());
        assert!(inside(root, "a/../../x").is_err());
    }

    /// The whole setup against the real downloads, into `HORIZON_SETUP_OUT` (about 600 MB):
    /// `HORIZON_SETUP_OUT=… cargo test -p horizon-client --lib ios_setup -- --ignored`.
    #[test]
    #[ignore]
    fn the_setup_downloads_and_assembles_the_resources() {
        let Some(out) = std::env::var_os("HORIZON_SETUP_OUT") else {
            return;
        };
        let out = PathBuf::from(out);
        let progress = Progress::default();
        run(&out.join("resources"), &out.join("work"), &progress).unwrap();
        assert!(prepared(&out.join("resources")));
        assert_eq!(progress.done.load(Relaxed), progress.total.load(Relaxed));
    }
}
