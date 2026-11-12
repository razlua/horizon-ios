//! Extra Settings (Horizon-iOS): the iOS port's own settings, on their own Options page
//! (`options_ui::Page::IosExtras`, opened from the title). Kept in `ios-settings.txt` in the game
//! folder, apart from `options.txt`, so the game's own files stay as they are.
use std::sync::Once;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

const FILE: &str = "ios-settings.txt";

/// The page's and its title button's name: the port's own settings, on iOS and on Android.
pub const TITLE: &str = if cfg!(target_os = "android") {
    "Extra Settings (Horizon-Android)"
} else {
    "Extra Settings (Horizon-iOS)"
};

static LOADED: Once = Once::new();
static SAFE_AREA: AtomicBool = AtomicBool::new(false);
static OPACITY: AtomicU32 = AtomicU32::new(100);
static LOOK_SPEED: AtomicU32 = AtomicU32::new(100);
static KEEP_AWAKE: AtomicBool = AtomicBool::new(false);

/// The page's settings, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    /// The GUI (HUD, menus, buttons) inside the screen's safe area, clear of the Dynamic Island
    /// and the home indicator; backgrounds still fill the screen.
    SafeArea,
    /// How opaque the touch buttons are.
    Opacity,
    /// How far a finger turns the view.
    LookSpeed,
    /// The screen doesn't dim or lock while the game runs.
    KeepAwake,
}

pub const ITEMS: [Item; 4] = [Item::SafeArea, Item::Opacity, Item::LookSpeed, Item::KeepAwake];

const OPACITIES: [u32; 5] = [100, 75, 50, 35, 20];
const LOOK_SPEEDS: [u32; 7] = [50, 75, 100, 125, 150, 200, 300];

fn load() {
    LOADED.call_once(|| {
        let Ok(text) = std::fs::read_to_string(FILE) else {
            return;
        };
        for line in text.lines() {
            let Some((key, value)) = line.trim_end_matches('\r').split_once(':') else {
                continue;
            };
            let number = || value.parse::<u32>().ok();
            match key {
                "safeArea" => SAFE_AREA.store(value == "true", Relaxed),
                "buttonOpacity" => {
                    if let Some(v) = number().filter(|v| OPACITIES.contains(v)) {
                        OPACITY.store(v, Relaxed);
                    }
                }
                "lookSpeed" => {
                    if let Some(v) = number().filter(|v| LOOK_SPEEDS.contains(v)) {
                        LOOK_SPEED.store(v, Relaxed);
                    }
                }
                "keepScreenOn" => KEEP_AWAKE.store(value == "true", Relaxed),
                _ => {}
            }
        }
    });
}

fn save() {
    let text = format!(
        "safeArea:{}\nbuttonOpacity:{}\nlookSpeed:{}\nkeepScreenOn:{}\n",
        SAFE_AREA.load(Relaxed),
        OPACITY.load(Relaxed),
        LOOK_SPEED.load(Relaxed),
        KEEP_AWAKE.load(Relaxed),
    );
    if let Err(e) = std::fs::write(FILE, text) {
        log::warn!("Horizon: {FILE} could not be saved: {e}");
    }
}

/// Whether the GUI keeps to the safe area (`hud::SAFE_INSETS`).
pub fn safe_area() -> bool {
    load();
    SAFE_AREA.load(Relaxed)
}

/// The touch buttons' opacity, 0.2 to 1.
pub fn opacity() -> f32 {
    load();
    OPACITY.load(Relaxed) as f32 / 100.0
}

/// How far a finger turns the view, against the default.
pub fn look_speed() -> f32 {
    load();
    LOOK_SPEED.load(Relaxed) as f32 / 100.0
}

/// Whether the screen stays on while the game runs.
pub fn keep_awake() -> bool {
    load();
    KEEP_AWAKE.load(Relaxed)
}

fn on_off(on: bool) -> &'static str {
    if on { "ON" } else { "OFF" }
}

/// The button's label.
pub fn label(item: Item) -> String {
    load();
    match item {
        Item::SafeArea => format!("Safe Area: {}", on_off(SAFE_AREA.load(Relaxed))),
        Item::Opacity => format!("Button Opacity: {}%", OPACITY.load(Relaxed)),
        Item::LookSpeed => format!("Touch Look Speed: {}%", LOOK_SPEED.load(Relaxed)),
        Item::KeepAwake => format!("Keep Screen On: {}", on_off(KEEP_AWAKE.load(Relaxed))),
    }
}

/// What the setting does, under the buttons while one is hovered or last tapped.
pub fn help(item: Item) -> &'static str {
    match item {
        Item::SafeArea if cfg!(target_os = "android") => {
            "Keeps the HUD and menus clear of the camera cutout and system bars."
        }
        Item::SafeArea => "Keeps the HUD and menus clear of the Dynamic Island and home bar.",
        Item::Opacity => "How see-through the touch buttons are.",
        Item::LookSpeed => "How far a finger turns the view.",
        Item::KeepAwake => "The screen doesn't dim or lock while Horizon is open.",
    }
}

fn next(list: &[u32], now: u32, back: bool) -> u32 {
    let i = list.iter().position(|v| *v == now).unwrap_or(0);
    let n = list.len();
    list[if back { (i + n - 1) % n } else { (i + 1) % n }]
}

/// Moves the setting to its next value (`back`: the one before) and saves.
pub fn cycle(item: Item, back: bool) {
    load();
    match item {
        Item::SafeArea => {
            SAFE_AREA.fetch_xor(true, Relaxed);
        }
        Item::Opacity => OPACITY.store(next(&OPACITIES, OPACITY.load(Relaxed), back), Relaxed),
        Item::LookSpeed => {
            LOOK_SPEED.store(next(&LOOK_SPEEDS, LOOK_SPEED.load(Relaxed), back), Relaxed)
        }
        Item::KeepAwake => {
            KEEP_AWAKE.fetch_xor(true, Relaxed);
        }
    }
    save();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_cycle_both_ways_and_wrap() {
        assert_eq!(next(&OPACITIES, 100, false), 75);
        assert_eq!(next(&OPACITIES, 20, false), 100);
        assert_eq!(next(&OPACITIES, 100, true), 20);
        assert_eq!(next(&LOOK_SPEEDS, 100, false), 125);
        // An unknown value starts from the first.
        assert_eq!(next(&LOOK_SPEEDS, 7, false), 75);
    }
}
