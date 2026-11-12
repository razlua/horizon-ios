//! What the phone builds (iOS and Android) ask of the system, under one name for both: the
//! calls themselves are `horizon_threads`' (`ios_*`, `android_sys`) and winit's.

/// Opens `url` in the system's browser. False when it could not.
pub(super) fn open_url(url: &str) -> bool {
    #[cfg(target_os = "ios")]
    {
        horizon_threads::ios_url::open_url(url)
    }
    #[cfg(target_os = "android")]
    {
        horizon_threads::android_sys::open_url(url)
    }
}

/// Keeps the screen on, or lets it sleep again.
pub(super) fn keep_awake(on: bool) {
    #[cfg(target_os = "ios")]
    horizon_threads::ios_display::keep_awake(on);
    #[cfg(target_os = "android")]
    if let Some(app) = crate::entry::android_app() {
        use winit::platform::android::activity::WindowManagerFlags;
        let (flag, none) = (WindowManagerFlags::KEEP_SCREEN_ON, WindowManagerFlags::empty());
        if on {
            app.set_window_flags(flag, none);
        } else {
            app.set_window_flags(none, flag);
        }
    }
}

/// How much of the screen's height the system keyboard covers now, 0 (none) to 1.
pub(super) fn keyboard_covered() -> f32 {
    #[cfg(target_os = "ios")]
    {
        horizon_threads::ios_keyboard::covered()
    }
    #[cfg(target_os = "android")]
    {
        horizon_threads::android_sys::keyboard_covered()
    }
}

/// The text on the system clipboard, if any.
pub(super) fn clipboard() -> Option<String> {
    #[cfg(target_os = "ios")]
    {
        horizon_threads::ios_pasteboard::get()
    }
    #[cfg(target_os = "android")]
    {
        horizon_threads::android_sys::clipboard_get()
    }
}

/// Each time the app comes to the screen: iOS follows the keyboard's frame and asks for the
/// screen's highest frame rate (Android draws at the screen's rate by itself).
pub(super) fn on_screen() {
    #[cfg(target_os = "ios")]
    {
        horizon_threads::ios_keyboard::watch();
        if let Some(rate) = horizon_threads::ios_display::high_frame_rate() {
            log::info!("Horizon iOS: asked for {rate} frames per second");
        }
    }
}

/// This build's version as the player sees it (`2026.11.12`), when the build says.
pub(super) fn version() -> Option<String> {
    #[cfg(target_os = "ios")]
    {
        horizon_threads::ios_bundle::info("CFBundleShortVersionString")
    }
    #[cfg(target_os = "android")]
    {
        option_env!("HORIZON_ANDROID_VERSION").map(str::to_owned)
    }
}

/// What the start screen calls the device.
pub(super) const DEVICE: &str = if cfg!(target_os = "ios") { "iPhone" } else { "phone" };
