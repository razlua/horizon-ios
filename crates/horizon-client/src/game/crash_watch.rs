//! iOS: what the last run was doing when it ended without leaving the screen. iOS ends an app
//! that goes over its memory with no word to it and nothing in the log, so every five seconds in
//! a world the game writes `home/running.json` (memory held and left, render distance, single
//! player or a server), and removes it when the app leaves the screen (which pauses and saves) or
//! closes from a Close button. Found at the next start, the run ended while playing: the launcher
//! says so, with a guess at why, and Send report shares `home/last-crash.txt` (that file, then the
//! end of that run's log).
use super::Game;
use serde_json::{Value, json};
use std::io::Read;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Every five seconds of game ticks.
const EVERY_TICKS: u64 = 100;
/// The lines of the last run's log in the report.
const LOG_LINES: usize = 400;
const MIB: u64 = 1024 * 1024;

/// What the last run was doing, found at this start, until the player dismisses it.
static LAST: Mutex<Option<Value>> = Mutex::new(None);

fn home() -> PathBuf {
    std::env::current_dir().unwrap_or_default().join("home")
}

fn running_path() -> PathBuf {
    home().join("running.json")
}

/// The report Send report shares (packaging/ios/swift, `shareCrashReport`).
pub(super) fn report_path() -> PathBuf {
    home().join("last-crash.txt")
}

/// The app left the screen or closes on purpose: an end after this isn't a crash.
pub(super) fn clear() {
    let _ = std::fs::remove_file(running_path());
}

/// At the start: whether the last run ended while playing. Its `running.json` becomes the
/// report, with the end of its log, and the launcher shows it until dismissed.
pub(super) fn take_last() {
    let path = running_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let _ = std::fs::remove_file(&path);
    let Ok(info) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    log::info!("Horizon iOS: the last run ended while playing: {info}");
    let mut report = String::from("Ferrite closed while playing.\n\n");
    report.push_str(&serde_json::to_string_pretty(&info).unwrap_or_default());
    report.push_str("\n\nThe end of that run's log:\n\n");
    report.push_str(&last_log_tail().unwrap_or_else(|| "(no log found)".into()));
    if let Err(e) = std::fs::write(report_path(), report) {
        log::warn!("Horizon iOS: the crash report couldn't be written: {e}");
    }
    *LAST.lock().unwrap_or_else(|e| e.into_inner()) = Some(info);
}

/// What the launcher shows about the last run, if it ended while playing.
pub(super) fn last() -> Option<Value> {
    LAST.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The player dismissed it.
pub(super) fn dismiss() {
    *LAST.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// The last run's log: the newest archive in `home/logs` (this start archived the old
/// `latest.log` as `<date>-<n>.log.gz`), its last lines.
fn last_log_tail() -> Option<String> {
    let dir = home().join("logs");
    let newest = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".log.gz"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .max_by_key(|(modified, _)| *modified)?
        .1;
    let mut text = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(newest).ok()?)
        .read_to_string(&mut text)
        .ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let from = lines.len().saturating_sub(LOG_LINES);
    Some(lines[from..].join("\n"))
}

impl Game<'_> {
    /// Every five seconds in a world: what a report needs if iOS ends the app now.
    pub(super) fn crash_watch_tick(&mut self) {
        if !self.ticks.is_multiple_of(EVERY_TICKS) {
            return;
        }
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let info = json!({
            "time": time,
            "in_ram_mb": horizon_client::ram_limit::used().unwrap_or(0) / MIB,
            "left_mb": horizon_threads::memory_limit::available_bytes().map(|b| b / MIB),
            "distance": self.render_distance_in_use(),
            "chosen_distance": self.settings.render_distance,
            "where": if self.remote.is_some() { "on a server" } else { "in single player" },
        });
        // Written beside, then moved over: never half a file.
        let path = running_path();
        let fresh = path.with_extension("json.new");
        if std::fs::write(&fresh, info.to_string()).is_ok() {
            let _ = std::fs::rename(&fresh, &path);
        }
    }
}
