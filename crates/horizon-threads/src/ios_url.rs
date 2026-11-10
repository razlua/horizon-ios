//! iOS: opening a web page in Safari (`UIApplication.openURL:options:completionHandler:`), which
//! the game crates, which forbid unsafe code, can't call themselves.

use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

/// Opens `url` in the system's browser. Call it on the main thread. False when `url` is not one.
pub fn open_url(url: &str) -> bool {
    let Ok(text) = std::ffi::CString::new(url) else {
        return false;
    };
    // SAFETY: Foundation's and UIKit's documented class methods, with the argument types they
    // take; a nil from any of them ends the call before it is used. `text` outlives the calls,
    // and the strings they return are autoreleased, owned by the run loop's pool.
    unsafe {
        let string: *mut AnyObject =
            msg_send![class!(NSString), stringWithUTF8String: text.as_ptr()];
        if string.is_null() {
            return false;
        }
        let link: *mut AnyObject = msg_send![class!(NSURL), URLWithString: string];
        if link.is_null() {
            return false;
        }
        let options: *mut AnyObject = msg_send![class!(NSDictionary), dictionary];
        let app: *mut AnyObject = msg_send![class!(UIApplication), sharedApplication];
        if app.is_null() {
            return false;
        }
        let no_handler: *mut AnyObject = std::ptr::null_mut();
        let _: () = msg_send![app, openURL: link, options: options, completionHandler: no_handler];
    }
    true
}
