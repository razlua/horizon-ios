//! iOS: controllers (Xbox, PlayStation, MFi, Switch Pro), a hardware keyboard and a mouse,
//! through Apple's GameController framework. gilrs has no iOS backend, and winit's iOS backend
//! sends no keys and no mouse. Everything here is polled once a frame from the game's loop on the
//! main thread, as the framework's objects want; only the mouse's movement comes in a handler,
//! added up until the next poll.
use std::ptr::NonNull;
use std::sync::Mutex;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2_game_controller::{
    GCController, GCControllerButtonInput, GCDevice, GCExtendedGamepad, GCKeyboard, GCMouse,
    GCMouseInput,
};

/// A controller's buttons and sticks now. Sticks and triggers -1..1 and 0..1, up positive.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pad {
    pub south: bool,
    pub east: bool,
    pub west: bool,
    pub north: bool,
    pub left_bumper: bool,
    pub right_bumper: bool,
    /// Options / Share / View / minus.
    pub select: bool,
    /// Menu / Options / plus.
    pub start: bool,
    pub left_stick_click: bool,
    pub right_stick_click: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub left_stick: [f32; 2],
    pub right_stick: [f32; 2],
    pub triggers: [f32; 2],
}

/// Whose button glyphs the pad has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Maker {
    Xbox,
    PlayStation,
    Nintendo,
    Other,
}

fn pressed(button: &GCControllerButtonInput) -> bool {
    // SAFETY: a property read on the main thread.
    unsafe { button.isPressed() }
}

fn optional(button: Option<Retained<GCControllerButtonInput>>) -> bool {
    button.is_some_and(|b| pressed(&b))
}

fn read(pad: &GCExtendedGamepad) -> Pad {
    // SAFETY: property reads of a connected controller's profile, on the main thread.
    unsafe {
        let dpad = pad.dpad();
        let left = pad.leftThumbstick();
        let right = pad.rightThumbstick();
        Pad {
            south: pressed(&pad.buttonA()),
            east: pressed(&pad.buttonB()),
            west: pressed(&pad.buttonX()),
            north: pressed(&pad.buttonY()),
            left_bumper: pressed(&pad.leftShoulder()),
            right_bumper: pressed(&pad.rightShoulder()),
            select: optional(pad.buttonOptions()),
            start: pressed(&pad.buttonMenu()),
            left_stick_click: optional(pad.leftThumbstickButton()),
            right_stick_click: optional(pad.rightThumbstickButton()),
            up: pressed(&dpad.up()),
            down: pressed(&dpad.down()),
            left: pressed(&dpad.left()),
            right: pressed(&dpad.right()),
            left_stick: [left.xAxis().value(), left.yAxis().value()],
            right_stick: [right.xAxis().value(), right.yAxis().value()],
            triggers: [pad.leftTrigger().value(), pad.rightTrigger().value()],
        }
    }
}

/// The maker, from the framework's product category ("DualSense", "Xbox One", ...).
pub fn maker_of(category: &str) -> Maker {
    let c = category.to_ascii_lowercase();
    if c.contains("xbox") {
        Maker::Xbox
    } else if c.contains("dualsense") || c.contains("dualshock") || c.contains("playstation") {
        Maker::PlayStation
    } else if c.contains("switch") || c.contains("joy-con") || c.contains("nintendo") {
        Maker::Nintendo
    } else {
        Maker::Other
    }
}

/// The controller played with last (the framework's `current`), with a full gamepad profile:
/// its state, its maker and its name. None without one.
pub fn pad() -> Option<(Pad, Maker, String)> {
    // SAFETY: class and property reads on the main thread.
    unsafe {
        let controller = GCController::current().or_else(|| GCController::controllers().firstObject())?;
        let gamepad = controller.extendedGamepad()?;
        let category = controller.productCategory().to_string();
        let name = controller.vendorName().map_or_else(|| category.clone(), |n| n.to_string());
        Some((read(&gamepad), maker_of(&category), name))
    }
}

/// The keys the game knows, by their USB HID usage (GameController's `GCKeyCode`s are those),
/// with the game's names for them.
pub const KEYS: &[(isize, &str)] = &[
    (4, "key.keyboard.a"), (5, "key.keyboard.b"), (6, "key.keyboard.c"), (7, "key.keyboard.d"),
    (8, "key.keyboard.e"), (9, "key.keyboard.f"), (10, "key.keyboard.g"), (11, "key.keyboard.h"),
    (12, "key.keyboard.i"), (13, "key.keyboard.j"), (14, "key.keyboard.k"), (15, "key.keyboard.l"),
    (16, "key.keyboard.m"), (17, "key.keyboard.n"), (18, "key.keyboard.o"), (19, "key.keyboard.p"),
    (20, "key.keyboard.q"), (21, "key.keyboard.r"), (22, "key.keyboard.s"), (23, "key.keyboard.t"),
    (24, "key.keyboard.u"), (25, "key.keyboard.v"), (26, "key.keyboard.w"), (27, "key.keyboard.x"),
    (28, "key.keyboard.y"), (29, "key.keyboard.z"),
    (30, "key.keyboard.1"), (31, "key.keyboard.2"), (32, "key.keyboard.3"), (33, "key.keyboard.4"),
    (34, "key.keyboard.5"), (35, "key.keyboard.6"), (36, "key.keyboard.7"), (37, "key.keyboard.8"),
    (38, "key.keyboard.9"), (39, "key.keyboard.0"),
    (40, "key.keyboard.enter"), (41, "key.keyboard.escape"), (42, "key.keyboard.backspace"),
    (43, "key.keyboard.tab"), (44, "key.keyboard.space"), (45, "key.keyboard.minus"),
    (46, "key.keyboard.equal"), (47, "key.keyboard.left.bracket"),
    (48, "key.keyboard.right.bracket"), (49, "key.keyboard.backslash"),
    (51, "key.keyboard.semicolon"), (52, "key.keyboard.apostrophe"),
    (53, "key.keyboard.grave.accent"), (54, "key.keyboard.comma"), (55, "key.keyboard.period"),
    (56, "key.keyboard.slash"), (57, "key.keyboard.caps.lock"),
    (58, "key.keyboard.f1"), (59, "key.keyboard.f2"), (60, "key.keyboard.f3"),
    (61, "key.keyboard.f4"), (62, "key.keyboard.f5"), (63, "key.keyboard.f6"),
    (64, "key.keyboard.f7"), (65, "key.keyboard.f8"), (66, "key.keyboard.f9"),
    (67, "key.keyboard.f10"), (68, "key.keyboard.f11"), (69, "key.keyboard.f12"),
    (76, "key.keyboard.delete"), (79, "key.keyboard.right"), (80, "key.keyboard.left"),
    (81, "key.keyboard.down"), (82, "key.keyboard.up"),
    (224, "key.keyboard.left.control"), (225, "key.keyboard.left.shift"),
    (226, "key.keyboard.left.alt"), (227, "key.keyboard.left.win"),
    (228, "key.keyboard.right.control"), (229, "key.keyboard.right.shift"),
    (230, "key.keyboard.right.alt"), (231, "key.keyboard.right.win"),
];

/// The keys held now on a hardware keyboard, by the game's names. Empty without a keyboard.
pub fn keys_down() -> Vec<&'static str> {
    // SAFETY: class and property reads on the main thread.
    unsafe {
        let Some(input) = GCKeyboard::coalescedKeyboard().and_then(|k| k.keyboardInput()) else {
            return Vec::new();
        };
        if !input.isAnyKeyPressed() {
            return Vec::new();
        }
        KEYS.iter()
            .filter(|(code, _)| input.buttonForKeyCode(*code).is_some_and(|b| b.isPressed()))
            .map(|(_, name)| *name)
            .collect()
    }
}

/// A mouse now: its movement since the last poll (right and down positive, in points) and its
/// buttons (left, right, middle).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mouse {
    pub moved: [f64; 2],
    pub buttons: [bool; 3],
}

/// The movement the handler added up since the last poll, and the mouse it was set on.
static MOVED: Mutex<[f64; 2]> = Mutex::new([0.0; 2]);
static HANDLED: Mutex<Option<usize>> = Mutex::new(None);

/// The mouse played with last, if there is one.
pub fn mouse() -> Option<Mouse> {
    // SAFETY: class and property reads on the main thread; the handler is set once a mouse.
    unsafe {
        let mouse = GCMouse::current()?;
        let input = mouse.mouseInput()?;
        let id = Retained::as_ptr(&input) as usize;
        let mut handled = HANDLED.lock().unwrap_or_else(|e| e.into_inner());
        if *handled != Some(id) {
            // GameController's movement is up positive; the game's, as a window's, down.
            let block = RcBlock::new(|_input: NonNull<GCMouseInput>, dx: f32, dy: f32| {
                let mut moved = MOVED.lock().unwrap_or_else(|e| e.into_inner());
                moved[0] += f64::from(dx);
                moved[1] -= f64::from(dy);
            });
            input.setMouseMovedHandler(RcBlock::as_ptr(&block));
            // The framework copies the handler; this one stays alive with it for good.
            std::mem::forget(block);
            *handled = Some(id);
        }
        let moved = std::mem::take(&mut *MOVED.lock().unwrap_or_else(|e| e.into_inner()));
        Some(Mouse {
            moved,
            buttons: [
                pressed(&input.leftButton()),
                optional(input.rightButton()),
                optional(input.middleButton()),
            ],
        })
    }
}

/// The game wants the iPad's pointer locked: in the world, with a mouse.
static POINTER_LOCKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// winit's view controller's `prefersPointerLocked`, added to its class (`lock_pointer`).
extern "C-unwind" fn prefers_pointer_locked(
    _this: &objc2::runtime::AnyObject,
    _sel: objc2::runtime::Sel,
) -> objc2::runtime::Bool {
    objc2::runtime::Bool::new(POINTER_LOCKED.load(std::sync::atomic::Ordering::Relaxed))
}

/// Locks the iPad's pointer to the game (hidden; the mouse only turns the view) or frees it.
/// iPadOS asks the window's view controller (winit's `WinitUIViewController`), which is given a
/// `prefersPointerLocked` the first time, then is told to ask again. iPadOS locks it only while
/// the app fills the screen. Nothing happens on an iPhone or without winit's class.
pub fn lock_pointer(lock: bool) {
    use objc2::runtime::{AnyClass, AnyObject, Imp};
    use objc2::{class, msg_send, sel};
    static ADDED: std::sync::Once = std::sync::Once::new();
    ADDED.call_once(|| {
        let Some(controller) = AnyClass::get(c"WinitUIViewController") else {
            return;
        };
        let answer: extern "C-unwind" fn(&AnyObject, objc2::runtime::Sel) -> objc2::runtime::Bool =
            prefers_pointer_locked;
        // SAFETY: an `(id, SEL) -> BOOL` method ("B@:" on 64-bit iOS) added to winit's view
        // controller class, which doesn't have one of its own; the cast is to the runtime's
        // untyped `IMP`.
        unsafe {
            let imp: Imp = std::mem::transmute(answer);
            objc2::ffi::class_addMethod(
                controller as *const AnyClass as *mut AnyClass,
                sel!(prefersPointerLocked),
                imp,
                c"B@:".as_ptr(),
            );
        }
    });
    if POINTER_LOCKED.swap(lock, std::sync::atomic::Ordering::Relaxed) == lock {
        return;
    }
    // SAFETY: UIKit calls on the main thread, where the game's loop runs; each pointer is checked.
    unsafe {
        let app: *mut AnyObject = msg_send![class!(UIApplication), sharedApplication];
        if app.is_null() {
            return;
        }
        let window: *mut AnyObject = msg_send![app, keyWindow];
        if window.is_null() {
            return;
        }
        let controller: *mut AnyObject = msg_send![window, rootViewController];
        if controller.is_null() {
            return;
        }
        let _: () = msg_send![controller, setNeedsUpdateOfPrefersPointerLocked];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makers_come_from_the_product_category() {
        assert_eq!(maker_of("DualSense"), Maker::PlayStation);
        assert_eq!(maker_of("DualShock 4"), Maker::PlayStation);
        assert_eq!(maker_of("Xbox One"), Maker::Xbox);
        assert_eq!(maker_of("Switch Pro Controller"), Maker::Nintendo);
        assert_eq!(maker_of("MFi"), Maker::Other);
    }

    #[test]
    fn every_key_code_has_one_name() {
        let mut codes: Vec<_> = KEYS.iter().map(|(c, _)| *c).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), KEYS.len());
        assert!(KEYS.iter().all(|(_, n)| n.starts_with("key.keyboard.")));
    }
}
