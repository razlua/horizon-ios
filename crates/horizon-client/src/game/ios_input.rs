//! iOS: a hardware keyboard and a mouse (horizon_threads::ios_gamecontroller), polled each frame.
//! winit's iOS backend sends neither. Keys go through the same path as the touch HUD's key
//! buttons (`key <name> down|up`), so F3, chat, hotbar keys and mods' keys behave as on a
//! keyboard. While a text box has the focus, the system's text input types into it, so only the
//! keys it doesn't take come from here. The mouse turns the view and its buttons attack and use in
//! the world; in menus the iPad's pointer already clicks as a touch. The controller itself is the
//! pad (pad.rs).
use super::Game;
use super::event_loop::EventLoopCtl;
use super::input_trace::InputOrigin;
use horizon_threads::ios_gamecontroller as gc;
use winit::event::{DeviceId, ElementState, MouseButton, WindowEvent};

/// Whether a text box takes this key itself (letters, digits, Return, Backspace, Space...).
/// Escape, Tab, Up and Down (chat's history) and the function keys still come from here.
fn typed_by_text_field(name: &str) -> bool {
    let function = name
        .strip_prefix("key.keyboard.f")
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    !function
        && !matches!(
            name,
            "key.keyboard.escape" | "key.keyboard.tab" | "key.keyboard.up" | "key.keyboard.down"
        )
}

/// A controller in use: a button, or a stick or trigger past the middle.
fn pad_moved(pad: &gc::Pad) -> bool {
    let buttons = [
        pad.south, pad.east, pad.west, pad.north, pad.left_bumper, pad.right_bumper, pad.select,
        pad.start, pad.left_stick_click, pad.right_stick_click, pad.up, pad.down, pad.left,
        pad.right,
    ];
    let far = |v: [f32; 2]| v[0].hypot(v[1]) > 0.35;
    buttons.into_iter().any(|b| b)
        || far(pad.left_stick)
        || far(pad.right_stick)
        || pad.triggers.iter().any(|t| *t > 0.35)
}

impl Game<'_> {
    /// Once a frame: the keyboard's keys and the mouse, and whether hardware drives the game.
    pub(super) fn ios_input_frame(&mut self, event_loop: &dyn EventLoopCtl) {
        if gc::pad().is_some_and(|(pad, _, _)| pad_moved(&pad)) {
            self.touch.hardware = true;
        }
        self.ios_keyboard_frame(event_loop);
        self.ios_mouse_frame(event_loop);
    }

    fn ios_keyboard_frame(&mut self, event_loop: &dyn EventLoopCtl) {
        let typing = self.touch.field_focused;
        let held: Vec<&'static str> = gc::keys_down()
            .into_iter()
            .filter(|name| !(typing && typed_by_text_field(name)))
            .collect();
        let before = std::mem::take(&mut self.touch.hardware_keys);
        if held.is_empty() && before.is_empty() {
            return;
        }
        for name in &held {
            if !before.contains(name) {
                self.replay_line(&format!("key {name} down"), event_loop);
            }
        }
        for name in &before {
            if !held.contains(name) {
                self.replay_line(&format!("key {name} up"), event_loop);
            }
        }
        if !held.is_empty() {
            self.touch.hardware = true;
        }
        self.touch.hardware_keys = held;
    }

    fn ios_mouse_frame(&mut self, event_loop: &dyn EventLoopCtl) {
        let Some(mouse) = gc::mouse() else {
            return;
        };
        let world = self.touch_world();
        if world && mouse.moved != [0.0, 0.0] && self.pending_save.is_none() {
            self.mouse.add_motion(mouse.moved[0], mouse.moved[1]);
            self.touch.hardware = true;
        }
        let Some(id) = self.window.as_ref().map(|w| w.id()) else {
            return;
        };
        let buttons = if world { mouse.buttons } else { [false; 3] };
        let before = self.touch.hardware_mouse;
        for (i, button) in [MouseButton::Left, MouseButton::Right, MouseButton::Middle]
            .into_iter()
            .enumerate()
        {
            if buttons[i] == before[i] {
                continue;
            }
            let state = if buttons[i] {
                ElementState::Pressed
            } else {
                ElementState::Released
            };
            let event = WindowEvent::MouseInput {
                device_id: DeviceId::dummy(),
                state,
                button,
            };
            self.handle_window_event(event_loop, id, event, InputOrigin::User);
            self.touch.hardware = true;
        }
        self.touch.hardware_mouse = buttons;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_box_types_letters_itself_but_escape_and_f_keys_still_come() {
        assert!(typed_by_text_field("key.keyboard.e"));
        assert!(typed_by_text_field("key.keyboard.f"));
        assert!(typed_by_text_field("key.keyboard.backspace"));
        assert!(!typed_by_text_field("key.keyboard.escape"));
        assert!(!typed_by_text_field("key.keyboard.f3"));
        assert!(!typed_by_text_field("key.keyboard.f12"));
        assert!(!typed_by_text_field("key.keyboard.up"));
    }
}
