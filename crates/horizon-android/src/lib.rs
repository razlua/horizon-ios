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
    }
    horizon_client::entry::android_main(app);
}
