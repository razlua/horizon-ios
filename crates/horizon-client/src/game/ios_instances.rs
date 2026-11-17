//! iOS: profiles (the launcher's word; instances in other launchers). A profile has its own
//! `options.txt` (keybinds and all), Horizon's video settings beside it, its servers list, hotbars
//! and offline name, in `instances/<name>` of the game folder, through Horizon's own player folder
//! (`logging::set_player_dir`, set when Play starts the game). Worlds, packs, mods, game files and
//! sign-ins stay shared. Default is the game folder itself, as before profiles: nothing moves for
//! a player who never makes one.
use std::path::PathBuf;

const DIR: &str = "instances";
/// The chosen profile's name; none (or a missing profile) is Default.
const CHOSEN: &str = "instance.txt";
/// What a new profile starts with, copied from Default: its settings and what goes with them.
const COPIED: &[&str] = &[
    "options.txt",
    "horizon-graphics.txt",
    "horizon-extras.txt",
    "servers.dat",
    "hotbar.nbt",
    "horizon-player.txt",
];

fn game_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_default()
}

/// A name a profile can have: 1 to 24 letters, digits, spaces, `-` or `_`, and not Default.
pub fn valid(name: &str) -> bool {
    let name = name.trim();
    (1..=24).contains(&name.chars().count())
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        && !name.eq_ignore_ascii_case("default")
}

/// The profiles there are, by name (Default isn't one of them).
pub(super) fn list() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(game_dir().join(DIR))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                .filter_map(|e| e.file_name().to_str().map(str::to_owned))
                .filter(|n| valid(n))
                .collect()
        })
        .unwrap_or_default();
    names.sort_by_key(|n| n.to_lowercase());
    names
}

/// The chosen profile, if it isn't Default.
pub(super) fn chosen() -> Option<String> {
    let name = std::fs::read_to_string(game_dir().join(CHOSEN)).ok()?;
    let name = name.trim().to_owned();
    (valid(&name) && game_dir().join(DIR).join(&name).is_dir()).then_some(name)
}

/// Chooses `name`, or Default.
pub(super) fn choose(name: Option<&str>) {
    let path = game_dir().join(CHOSEN);
    let written = match name.filter(|n| valid(n)) {
        Some(name) => std::fs::write(&path, name.trim()),
        None => std::fs::remove_file(&path).or(Ok(())),
    };
    if let Err(e) = written {
        log::warn!("Horizon iOS: the profile couldn't be chosen: {e}");
    }
}

/// A new profile named `name`, starting with Default's settings, and chosen.
pub(super) fn create(name: &str) {
    let name = name.trim();
    if !valid(name) {
        return log::warn!("Horizon iOS: {name:?} can't be a profile's name");
    }
    let dir = game_dir().join(DIR).join(name);
    if dir.exists() {
        return choose(Some(name));
    }
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return log::warn!("Horizon iOS: the profile {name} couldn't be made: {e}");
    }
    for file in COPIED {
        let from = game_dir().join(file);
        if from.is_file() {
            let _ = std::fs::copy(&from, dir.join(file));
        }
    }
    log::info!("Horizon iOS: made the profile {name}");
    choose(Some(name));
}

/// Deletes the profile `name` (its settings; the shared worlds stay). Default is chosen if it was.
pub(super) fn delete(name: &str) {
    let name = name.trim();
    if !valid(name) {
        return;
    }
    if chosen().as_deref() == Some(name) {
        choose(None);
    }
    if let Err(e) = std::fs::remove_dir_all(game_dir().join(DIR).join(name)) {
        log::warn!("Horizon iOS: the profile {name} couldn't be deleted: {e}");
    }
}

/// The chosen profile's folder for Play to give the game, if it isn't Default.
pub(super) fn player_dir() -> Option<PathBuf> {
    chosen().map(|name| game_dir().join(DIR).join(name))
}

/// The `options.txt` the launcher's Settings tab shows and changes: the chosen profile's.
pub(super) fn options_path() -> PathBuf {
    match player_dir() {
        Some(dir) => dir.join("options.txt"),
        None => super::Game::options_path(),
    }
}

#[cfg(test)]
mod tests {
    use super::valid;

    #[test]
    fn profile_names_are_short_plain_and_not_default() {
        assert!(valid("PvP"));
        assert!(valid("Survival 2"));
        assert!(valid("my_world-1"));
        assert!(!valid(""));
        assert!(!valid("   "));
        assert!(!valid("Default"));
        assert!(!valid("../saves"));
        assert!(!valid("a/b"));
        assert!(!valid("a name that is far too long to fit"));
    }
}
