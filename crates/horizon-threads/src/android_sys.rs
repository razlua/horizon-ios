//! Android: the system calls the phone build needs (the browser, the clipboard, how much of the
//! screen the keyboard covers), through JNI on the app's activity. The game crates forbid unsafe
//! code, so they can't call them themselves. android-activity gives the Java VM and the activity
//! to `ndk_context` before `android_main` runs.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};

use jni::objects::{JObject, JString, JValue};
use jni::{JNIEnv, JavaVM};

type Jni<T> = jni::errors::Result<T>;

/// Runs `f` with this thread's JNI environment and the activity. None when the VM can't be
/// reached or a call threw (the exception is logged by the VM and cleared).
fn with_activity<T>(f: impl FnOnce(&mut JNIEnv, &JObject) -> Jni<T>) -> Option<T> {
    let context = ndk_context::android_context();
    // SAFETY: the pointers android-activity registered: the process's Java VM and a global
    // reference to the activity, both alive as long as the process.
    let vm = unsafe { JavaVM::from_raw(context.vm().cast()) }.ok()?;
    // SAFETY: as above; the reference is global and is not deleted through this handle.
    let activity = unsafe { JObject::from_raw(context.context().cast()) };
    // The thread stays attached: these are called every frame while the keyboard is up.
    let mut env = vm.attach_current_thread_permanently().ok()?;
    let result = env.with_local_frame(32, |env| f(env, &activity));
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
    result.ok()
}

/// Opens `url` in the system's browser (an `ACTION_VIEW` intent). False when it could not start.
pub fn open_url(url: &str) -> bool {
    with_activity(|env, activity| {
        let text = env.new_string(url)?;
        let uri = env
            .call_static_method(
                "android/net/Uri",
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&text)],
            )?
            .l()?;
        let action = env.new_string("android.intent.action.VIEW")?;
        let intent = env.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;Landroid/net/Uri;)V",
            &[JValue::Object(&action), JValue::Object(&uri)],
        )?;
        env.call_method(
            activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;
        Ok(())
    })
    .is_some()
}

fn clipboard_manager<'a>(env: &mut JNIEnv<'a>, activity: &JObject) -> Jni<JObject<'a>> {
    let name = env.new_string("clipboard")?;
    env.call_method(
        activity,
        "getSystemService",
        "(Ljava/lang/String;)Ljava/lang/Object;",
        &[JValue::Object(&name)],
    )?
    .l()
}

/// The text on the clipboard, if any. Android gives it only to the app in front.
pub fn clipboard_get() -> Option<String> {
    with_activity(|env, activity| {
        let manager = clipboard_manager(env, activity)?;
        if manager.is_null() {
            return Ok(None);
        }
        let clip = env
            .call_method(&manager, "getPrimaryClip", "()Landroid/content/ClipData;", &[])?
            .l()?;
        if clip.is_null() {
            return Ok(None);
        }
        if env.call_method(&clip, "getItemCount", "()I", &[])?.i()? < 1 {
            return Ok(None);
        }
        let item = env
            .call_method(&clip, "getItemAt", "(I)Landroid/content/ClipData$Item;", &[JValue::Int(0)])?
            .l()?;
        let chars = env
            .call_method(
                &item,
                "coerceToText",
                "(Landroid/content/Context;)Ljava/lang/CharSequence;",
                &[JValue::Object(activity)],
            )?
            .l()?;
        if chars.is_null() {
            return Ok(None);
        }
        let string = env.call_method(&chars, "toString", "()Ljava/lang/String;", &[])?.l()?;
        let string = JString::from(string);
        let text: String = env.get_string(&string)?.into();
        Ok(Some(text))
    })
    .flatten()
}

/// Puts `text` on the clipboard.
pub fn clipboard_set(text: &str) {
    with_activity(|env, activity| {
        let manager = clipboard_manager(env, activity)?;
        if manager.is_null() {
            return Ok(());
        }
        let label = env.new_string("Horizon")?;
        let text = env.new_string(text)?;
        let clip = env
            .call_static_method(
                "android/content/ClipData",
                "newPlainText",
                "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
                &[JValue::Object(&label), JValue::Object(&text)],
            )?
            .l()?;
        env.call_method(
            &manager,
            "setPrimaryClip",
            "(Landroid/content/ClipData;)V",
            &[JValue::Object(&clip)],
        )?;
        Ok(())
    });
}

/// Shows or hides the system keyboard, asked of the input method manager for the window's own
/// view and without the "implicit" flag, which a window with no text editor in it needs.
pub fn show_keyboard(show: bool) {
    with_activity(|env, activity| {
        let name = env.new_string("input_method")?;
        let manager = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&name)],
            )?
            .l()?;
        let window = env.call_method(activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
        if manager.is_null() || window.is_null() {
            return Ok(());
        }
        let decor = env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?.l()?;
        if decor.is_null() {
            return Ok(());
        }
        if show {
            env.call_method(
                &manager,
                "showSoftInput",
                "(Landroid/view/View;I)Z",
                &[JValue::Object(&decor), JValue::Int(0)],
            )?;
        } else {
            let token = env
                .call_method(&decor, "getWindowToken", "()Landroid/os/IBinder;", &[])?
                .l()?;
            env.call_method(
                &manager,
                "hideSoftInputFromWindow",
                "(Landroid/os/IBinder;I)Z",
                &[JValue::Object(&token), JValue::Int(0)],
            )?;
        }
        Ok(())
    });
}

/// Asks the player for the microphone (`RECORD_AUDIO`), unless the app already has it. Android
/// shows its own question; the voice chat opens the microphone once the answer is yes.
pub fn request_microphone() {
    with_activity(|env, activity| {
        let permission = env.new_string("android.permission.RECORD_AUDIO")?;
        let state = env
            .call_method(
                activity,
                "checkSelfPermission",
                "(Ljava/lang/String;)I",
                &[JValue::Object(&permission)],
            )?
            .i()?;
        // PackageManager.PERMISSION_GRANTED.
        if state == 0 {
            return Ok(());
        }
        let wanted = env.new_object_array(1, "java/lang/String", &permission)?;
        env.call_method(
            activity,
            "requestPermissions",
            "([Ljava/lang/String;I)V",
            &[JValue::Object(&wanted), JValue::Int(1)],
        )?;
        Ok(())
    });
}

/// How much of the window's height the keyboard covers now, 0 (none) to 1. Asked of the system
/// at most ten times a second: the game asks every frame while chat is open.
pub fn keyboard_covered() -> f32 {
    static AT: AtomicU64 = AtomicU64::new(0);
    static COVERED: AtomicU32 = AtomicU32::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    if now.wrapping_sub(AT.load(Relaxed)) >= 100 {
        AT.store(now, Relaxed);
        let covered = with_activity(covered_now).unwrap_or(0.0);
        COVERED.store(covered.to_bits(), Relaxed);
    }
    f32::from_bits(COVERED.load(Relaxed))
}

fn covered_now(env: &mut JNIEnv, activity: &JObject) -> Jni<f32> {
    let window = env.call_method(activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
    if window.is_null() {
        return Ok(0.0);
    }
    let decor = env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?.l()?;
    if decor.is_null() {
        return Ok(0.0);
    }
    let height = env.call_method(&decor, "getHeight", "()I", &[])?.i()?;
    if height <= 0 {
        return Ok(0.0);
    }
    // The part of the window still visible: the keyboard takes its bottom away.
    let rect = env.new_object("android/graphics/Rect", "()V", &[])?;
    env.call_method(
        &decor,
        "getWindowVisibleDisplayFrame",
        "(Landroid/graphics/Rect;)V",
        &[JValue::Object(&rect)],
    )?;
    let bottom = env.get_field(&rect, "bottom", "I")?.i()?;
    let mut hidden = (height - bottom).max(0);
    // Android 11 and later say what the keyboard itself covers, whatever the window's mode.
    if let Some(ime) = ime_inset(env, &decor) {
        hidden = hidden.max(ime);
    }
    // A sliver is the navigation bar, not a keyboard.
    let share = hidden as f32 / height as f32;
    Ok(if share < 0.15 { 0.0 } else { share.min(1.0) })
}

/// `getRootWindowInsets().getInsets(WindowInsets.Type.ime()).bottom`, on Android 11 and later.
fn ime_inset(env: &mut JNIEnv, decor: &JObject) -> Option<i32> {
    let found = (|| -> Jni<i32> {
        let insets = env
            .call_method(decor, "getRootWindowInsets", "()Landroid/view/WindowInsets;", &[])?
            .l()?;
        if insets.is_null() {
            return Ok(0);
        }
        let ime = env.call_static_method("android/view/WindowInsets$Type", "ime", "()I", &[])?.i()?;
        let sides = env
            .call_method(&insets, "getInsets", "(I)Landroid/graphics/Insets;", &[JValue::Int(ime)])?
            .l()?;
        env.get_field(&sides, "bottom", "I")?.i()
    })();
    // An older Android has no such class or method: that is an exception to clear, not an error.
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    found.ok()
}
