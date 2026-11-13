//! iOS: the SwiftUI launcher (packaging/ios/swift/Launcher.swift), shown over the game's window
//! before the game starts. It is in the app only when the build compiled it
//! (`horizon_swift_ui`, which build.rs sets when `HORIZON_IOS_SWIFT_LIB` names its library);
//! without it every call here does nothing and the game's own start screen shows. The game
//! crates forbid unsafe code, so these are their safe calls into it. Call them on the main
//! thread, the event loop's.

use std::sync::Mutex;

/// The launcher's actions, as Swift sent them, for the game to take on its next turn.
static ACTIONS: Mutex<Vec<String>> = Mutex::new(Vec::new());

#[cfg(horizon_swift_ui)]
mod swift {
    use std::ffi::c_char;
    unsafe extern "C" {
        pub fn hz_launcher_show() -> bool;
        pub fn hz_launcher_hide();
        pub fn hz_launcher_update(json: *const c_char);
    }
}

/// Whether this build has the launcher.
pub fn available() -> bool {
    cfg!(horizon_swift_ui)
}

/// Shows the launcher over the game's window. False without the launcher, or without a window.
pub fn show() -> bool {
    // SAFETY: Launcher.swift's function, which takes nothing; on the main thread.
    #[cfg(horizon_swift_ui)]
    return unsafe { swift::hz_launcher_show() };
    #[cfg(not(horizon_swift_ui))]
    false
}

/// Takes the launcher away.
pub fn hide() {
    // SAFETY: as `show`.
    #[cfg(horizon_swift_ui)]
    unsafe {
        swift::hz_launcher_hide()
    }
}

/// Sends the launcher the game's state, as JSON.
pub fn update(json: &str) {
    #[cfg(horizon_swift_ui)]
    if let Ok(text) = std::ffi::CString::new(json) {
        // SAFETY: a NUL-terminated string that outlives the call, which copies it.
        unsafe { swift::hz_launcher_update(text.as_ptr()) }
    }
    #[cfg(not(horizon_swift_ui))]
    let _ = json;
}

/// The actions the player took in the launcher since the last call, oldest first.
pub fn take_actions() -> Vec<String> {
    std::mem::take(&mut *ACTIONS.lock().unwrap_or_else(|e| e.into_inner()))
}

/// Called by Launcher.swift when the player taps something.
#[cfg(horizon_swift_ui)]
#[unsafe(no_mangle)]
pub extern "C" fn hz_launcher_action(action: *const std::ffi::c_char) {
    if action.is_null() {
        return;
    }
    // SAFETY: Swift passes a NUL-terminated string that lives for the call.
    let action = unsafe { std::ffi::CStr::from_ptr(action) }.to_string_lossy().into_owned();
    ACTIONS.lock().unwrap_or_else(|e| e.into_inner()).push(action);
}
