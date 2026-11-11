//! iOS: the system clipboard (`UIPasteboard.generalPasteboard`), which the game crates, which
//! forbid unsafe code, can't call themselves. iOS asks the player before an app reads it.

use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use std::ffi::{CStr, CString, c_char};

fn pasteboard() -> *mut AnyObject {
    // SAFETY: UIKit's documented class method; it returns nil or the shared pasteboard.
    unsafe { msg_send![class!(UIPasteboard), generalPasteboard] }
}

/// The text on the clipboard, if any.
pub fn get() -> Option<String> {
    let board = pasteboard();
    if board.is_null() {
        return None;
    }
    // SAFETY: documented methods on a live pasteboard; the string is autoreleased and read before
    // this returns, and `UTF8String` points into it.
    unsafe {
        let string: *mut AnyObject = msg_send![board, string];
        if string.is_null() {
            return None;
        }
        let utf8: *const c_char = msg_send![string, UTF8String];
        (!utf8.is_null()).then(|| CStr::from_ptr(utf8).to_string_lossy().into_owned())
    }
}

/// Puts `text` on the clipboard.
pub fn set(text: &str) {
    let board = pasteboard();
    let Ok(text) = CString::new(text.replace('\0', "")) else {
        return;
    };
    if board.is_null() {
        return;
    }
    // SAFETY: documented methods with the argument types they take; `text` outlives the calls.
    unsafe {
        let string: *mut AnyObject = msg_send![class!(NSString), stringWithUTF8String: text.as_ptr()];
        if !string.is_null() {
            let _: () = msg_send![board, setString: string];
        }
    }
}
