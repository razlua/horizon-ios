//! iOS: the app's own Info.plist values, such as its version and build number.

use std::ffi::{CStr, CString, c_char};

use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send};

/// The main bundle's Info.plist string for `key` (`CFBundleVersion`, say), if it has one.
pub fn info(key: &str) -> Option<String> {
    let key = CString::new(key).ok()?;
    // SAFETY: Foundation's documented methods with the types they take and return; nil and
    // non-string values end the call before they are used. The UTF-8 pointer lives as long as
    // the string, which the bundle keeps, and is copied at once.
    unsafe {
        let bundle: *mut AnyObject = msg_send![class!(NSBundle), mainBundle];
        if bundle.is_null() {
            return None;
        }
        let name: *mut AnyObject = msg_send![class!(NSString), stringWithUTF8String: key.as_ptr()];
        if name.is_null() {
            return None;
        }
        let value: *mut AnyObject = msg_send![bundle, objectForInfoDictionaryKey: name];
        if value.is_null() {
            return None;
        }
        let string: Bool = msg_send![value, isKindOfClass: class!(NSString)];
        if !string.as_bool() {
            return None;
        }
        let text: *const c_char = msg_send![value, UTF8String];
        if text.is_null() {
            return None;
        }
        Some(CStr::from_ptr(text).to_string_lossy().into_owned())
    }
}
