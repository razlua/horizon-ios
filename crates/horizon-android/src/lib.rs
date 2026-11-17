//! The Android app's native library. `NativeActivity` loads it and android-activity's glue calls
//! `android_main` on a thread of its own, which is the game's main thread from then on.
//!
//! The game folder is the app's own folder on shared storage (`Android/data/<package>/files`,
//! which a file manager or a computer shows), and it is the current directory, as the game
//! folder is on every platform. Sign-ins are in the app's private storage (`HOME`).
#![cfg(target_os = "android")]

use winit::platform::android::activity::AndroidApp;

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    let private = app.internal_data_path();
    let shared = app.external_data_path().or_else(|| private.clone());
    if let Some(private) = &private {
        let temp = private.join("tmp");
        let _ = std::fs::create_dir_all(&temp);
        // SAFETY: the game's first lines, before it starts any thread that reads the
        // environment; Android's own threads don't read these.
        unsafe {
            std::env::set_var("HOME", private);
            std::env::set_var("TMPDIR", &temp);
        }
    }
    if let Some(shared) = &shared {
        let _ = std::fs::create_dir_all(shared);
        let _ = std::env::set_current_dir(shared);
        // The Chromebook build (horizon-chrome.apk) carries an asset that says so.
        if app.asset_manager().open(c"chromebook").is_some() {
            chromebook_settings(shared);
            // Frames over 50 ms go to the log with what they spent their time in, so a lag
            // spike can be traced from the log a player sends.
            // SAFETY: still before the game starts any thread that reads the environment.
            unsafe {
                std::env::set_var("HORIZON_SPIKES", "50");
            }
        }
    }
    horizon_client::entry::android_main(app);
}

/// The Chromebook build's light settings, once (`.horizon-chromebook` says it's done), for weak
/// Chromebook GPUs (a Kompanio 540's Mali-G57 ran the defaults at about 20 fps): the world drawn
/// at 70% of the screen's resolution, render distance 6, Fast graphics, no clouds, minimal
/// particles, no smooth lighting, shadows or biome blend, 60 fps, and the phones' culling. The
/// player changes any of it in Options afterwards.
fn chromebook_settings(dir: &std::path::Path) {
    let marker = dir.join(".horizon-chromebook");
    if marker.exists() {
        return;
    }
    let path = dir.join("options.txt");
    let mut s = horizon_client::settings::Settings::load(&path);
    s.render_distance = 6;
    s.simulation_distance = 5;
    s.max_fps = 60;
    s.vsync = false;
    s.graphics_preset = "fast".into();
    s.smooth_lighting = false;
    s.clouds = "false".into();
    s.particles = 2;
    s.entity_shadows = false;
    s.biome_blend = 0;
    s.mipmap_levels = 0;
    s.horizon.resolution_scale = 70;
    s.horizon.phone_savings();
    s.save(&path);
    let _ = std::fs::write(marker, "");
}
