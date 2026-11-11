//! iOS: how much of the screen the system keyboard covers, from UIKit's keyboard notifications,
//! so the chat can move up above it while you type. winit doesn't say.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

use block2::RcBlock;
use objc2::encode::{Encode, Encoding};
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

/// The covered part of the screen's height, 0 to 1, as `f32` bits.
static COVERED: AtomicU32 = AtomicU32::new(0);

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

// SAFETY: the layouts of Core Graphics' `CGPoint`, `CGSize` and `CGRect` on 64-bit iOS: `double`s.
unsafe impl Encode for CGPoint {
    const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
}
unsafe impl Encode for CGSize {
    const ENCODING: Encoding = Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
}
unsafe impl Encode for CGRect {
    const ENCODING: Encoding = Encoding::Struct("CGRect", &[CGPoint::ENCODING, CGSize::ENCODING]);
}

#[link(name = "UIKit", kind = "framework")]
unsafe extern "C" {
    static UIKeyboardWillChangeFrameNotification: *mut AnyObject;
    static UIKeyboardWillHideNotification: *mut AnyObject;
    static UIKeyboardFrameEndUserInfoKey: *mut AnyObject;
}

/// How much of the screen's height the keyboard covers now, 0 (none) to 1.
pub fn covered() -> f32 {
    f32::from_bits(COVERED.load(Relaxed))
}

/// The part of the screen under a keyboard notification's end frame.
///
/// # Safety
/// `note` is an `NSNotification` or null, on the main thread.
unsafe fn covered_by(note: *mut AnyObject) -> f32 {
    // SAFETY: Foundation's and UIKit's documented methods with the types they take and return;
    // nil ends the call before it is used.
    unsafe {
        if note.is_null() {
            return 0.0;
        }
        let info: *mut AnyObject = msg_send![note, userInfo];
        if info.is_null() {
            return 0.0;
        }
        let value: *mut AnyObject = msg_send![info, objectForKey: UIKeyboardFrameEndUserInfoKey];
        if value.is_null() {
            return 0.0;
        }
        let frame: CGRect = msg_send![value, CGRectValue];
        let screen: *mut AnyObject = msg_send![class!(UIScreen), mainScreen];
        if screen.is_null() {
            return 0.0;
        }
        let bounds: CGRect = msg_send![screen, bounds];
        let height = bounds.size.height;
        if height <= 0.0 {
            return 0.0;
        }
        ((height - frame.origin.y) / height).clamp(0.0, 1.0) as f32
    }
}

/// Starts following the keyboard's frame, once. Call it on the main thread.
pub fn watch() {
    static WATCHING: AtomicBool = AtomicBool::new(false);
    if WATCHING.swap(true, Relaxed) {
        return;
    }
    let changed = RcBlock::new(|note: *mut AnyObject| {
        // SAFETY: UIKit sends an `NSNotification`, on the main queue.
        let covered = unsafe { covered_by(note) };
        COVERED.store(covered.to_bits(), Relaxed);
    });
    let hidden = RcBlock::new(|_: *mut AnyObject| COVERED.store(0f32.to_bits(), Relaxed));
    // SAFETY: Foundation's documented `addObserverForName:object:queue:usingBlock:` on the
    // default center, with the main queue, so the blocks run on the main thread. The center
    // copies the blocks; the observers are retained for the whole run.
    unsafe {
        let center: *mut AnyObject = msg_send![class!(NSNotificationCenter), defaultCenter];
        let queue: *mut AnyObject = msg_send![class!(NSOperationQueue), mainQueue];
        if center.is_null() || queue.is_null() {
            return;
        }
        for (name, block) in [
            (UIKeyboardWillChangeFrameNotification, &changed),
            (UIKeyboardWillHideNotification, &hidden),
        ] {
            let nothing: *mut AnyObject = std::ptr::null_mut();
            let observer: *mut AnyObject = msg_send![
                center,
                addObserverForName: name,
                object: nothing,
                queue: queue,
                usingBlock: &**block
            ];
            if !observer.is_null() {
                let _: *mut AnyObject = msg_send![observer, retain];
            }
        }
    }
}
