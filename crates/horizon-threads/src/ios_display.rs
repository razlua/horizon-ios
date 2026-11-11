//! iOS: the frame rate. Core Animation runs an app at 60 Hz unless something in it asks for more
//! with a display link's preferred frame rate range (ProMotion screens go to 120 Hz, with
//! `CADisableMinimumFrameDurationOnPhone` in the Info.plist). winit draws without one, so the game
//! stayed at 60 whatever its own frame limit said. One display link asks for the screen's
//! maximum, for the whole run.

use objc2::encode::{Encode, Encoding};
use objc2::runtime::AnyObject;
use objc2::{class, msg_send, sel};

/// `CAFrameRateRange`.
#[repr(C)]
#[derive(Clone, Copy)]
struct FrameRateRange {
    minimum: f32,
    maximum: f32,
    preferred: f32,
}

// SAFETY: the layout of `CAFrameRateRange`: three `float`s.
unsafe impl Encode for FrameRateRange {
    const ENCODING: Encoding = Encoding::Struct(
        "CAFrameRateRange",
        &[f32::ENCODING, f32::ENCODING, f32::ENCODING],
    );
}

#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {
    static NSRunLoopCommonModes: *mut AnyObject;
}

/// Keeps the screen on (`UIApplication.idleTimerDisabled`), or lets it sleep again. Call it on
/// the main thread.
pub fn keep_awake(on: bool) {
    // SAFETY: UIKit's documented property on the shared application, on the main thread.
    unsafe {
        let app: *mut AnyObject = msg_send![class!(UIApplication), sharedApplication];
        if !app.is_null() {
            let _: () = msg_send![app, setIdleTimerDisabled: objc2::runtime::Bool::new(on)];
        }
    }
}

/// Asks for the screen's highest frame rate, once. Call it on the main thread. The rate the
/// system picked, or `None` when it could not be asked.
pub fn high_frame_rate() -> Option<i64> {
    static ASKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if ASKED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return None;
    }
    // SAFETY: UIKit's, Core Animation's and Foundation's documented methods with the argument
    // types they take, on the main thread; nil results end the call before they are used. The
    // display link and its target stay for the whole run (the run loop holds the link).
    unsafe {
        let screen: *mut AnyObject = msg_send![class!(UIScreen), mainScreen];
        if screen.is_null() {
            return None;
        }
        let max: isize = msg_send![screen, maximumFramesPerSecond];
        // Any object answers `hash`; the link only needs a target that takes its call.
        let target: *mut AnyObject = msg_send![class!(NSObject), new];
        let link: *mut AnyObject =
            msg_send![class!(CADisplayLink), displayLinkWithTarget: target, selector: sel!(hash)];
        if link.is_null() {
            return None;
        }
        let rate = max.max(60) as f32;
        let range = FrameRateRange {
            minimum: 60.0_f32.min(rate),
            maximum: rate,
            preferred: rate,
        };
        let _: () = msg_send![link, setPreferredFrameRateRange: range];
        let run_loop: *mut AnyObject = msg_send![class!(NSRunLoop), mainRunLoop];
        let _: () = msg_send![link, addToRunLoop: run_loop, forMode: NSRunLoopCommonModes];
        Some(max as i64)
    }
}
