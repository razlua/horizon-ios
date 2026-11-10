//! Touch controls (Horizon's own; the iOS build, docs/ios.md): fingers on the glass become the
//! inputs the game already takes.
//!
//! - **In a screen** (title, menus, inventories) a finger is the mouse. A tap is a left click
//!   where it lands. A quick vertical swipe scrolls. A sideways drag, or a drag after holding
//!   still for a moment, is a left button drag (sliders, scroll bars). A second finger tapped
//!   while the first rests is a right click at the first finger.
//! - **In the world** the left of the screen is a floating move stick, the buttons are the
//!   controller's (pad.rs merges them as one more pad), and a finger anywhere else turns the view
//!   like a grabbed mouse. A finger on a button turns the view too, so mining follows the thumb.
//!
//! The logic here is plain data in and out (`Controls::touch` returns what the game should do),
//! so it is tested without a window.

use super::event_loop::EventLoopCtl;
use super::{Game, Screen, input_trace};
use horizon_client::gamepad::{Button, PadState};
use horizon_client::hud::{HudBatch, rgba};
use std::time::{Duration, Instant};
use winit::event::{
    DeviceId, ElementState, MouseButton, MouseScrollDelta, Touch, TouchPhase, WindowEvent,
};
use winit::window::{Window, WindowId};

/// A press held this long without moving becomes a drag instead of a scroll.
const HOLD: Duration = Duration::from_millis(220);
/// Finger travel per degree-ish of turn, against a mouse's counts.
const LOOK_SCALE: f64 = 0.55;
/// The move stick past this many radii holds sprint.
const SPRINT_AT: f32 = 1.6;

/// What a button on the glass does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Key {
    Pad(Button),
    /// 0 is the left trigger (use), 1 the right (attack).
    Trigger(usize),
    /// Keyboard keys (`horizon-ctl key`'s names), held in order while the finger stays: F1, F3,
    /// or a debug combination such as F3 then G.
    Keys(&'static [&'static str]),
}

/// One action a touch button can have.
pub(super) struct Action {
    /// Its name in `touch-controls.json`.
    pub id: &'static str,
    /// On the button.
    pub label: &'static str,
    /// In Edit HUD's Add list.
    pub name: &'static str,
    pub key: Key,
}

const fn action(id: &'static str, label: &'static str, name: &'static str, key: Key) -> Action {
    Action {
        id,
        label,
        name,
        key,
    }
}

/// Every action, in Edit HUD's order. The controller's buttons go through the virtual pad (their
/// bindings: docs/controllers.md); the others press keyboard keys.
pub(super) const ACTIONS: &[Action] = &[
    action("jump", "Jump", "Jump", Key::Pad(Button::South)),
    action("sneak", "Sneak", "Sneak", Key::Pad(Button::RightStick)),
    action("use", "Use", "Use / place", Key::Trigger(0)),
    action("hit", "Hit", "Attack / mine", Key::Trigger(1)),
    action("inventory", "Inv", "Inventory", Key::Pad(Button::North)),
    action("chat", "Chat", "Chat", Key::Pad(Button::DPadRight)),
    action("drop", "Drop", "Drop item", Key::Pad(Button::East)),
    action("camera", "Cam", "Camera (F5)", Key::Pad(Button::DPadUp)),
    action("hotbar_previous", "<", "Hotbar left", Key::Pad(Button::LeftBumper)),
    action("hotbar_next", ">", "Hotbar right", Key::Pad(Button::RightBumper)),
    action("sprint", "Run", "Sprint", Key::Pad(Button::LeftStick)),
    action("swap_hands", "Swap", "Swap hands", Key::Pad(Button::West)),
    action("pick_block", "Pick", "Pick block", Key::Pad(Button::DPadLeft)),
    action("advancements", "Adv", "Advancements", Key::Pad(Button::DPadDown)),
    action("player_list", "List", "Player list", Key::Pad(Button::Select)),
    action("command", "/", "Command (/)", Key::Keys(&["slash"])),
    action("hotbar_1", "1", "Hotbar slot 1", Key::Keys(&["1"])),
    action("hotbar_2", "2", "Hotbar slot 2", Key::Keys(&["2"])),
    action("hotbar_3", "3", "Hotbar slot 3", Key::Keys(&["3"])),
    action("hotbar_4", "4", "Hotbar slot 4", Key::Keys(&["4"])),
    action("hotbar_5", "5", "Hotbar slot 5", Key::Keys(&["5"])),
    action("hotbar_6", "6", "Hotbar slot 6", Key::Keys(&["6"])),
    action("hotbar_7", "7", "Hotbar slot 7", Key::Keys(&["7"])),
    action("hotbar_8", "8", "Hotbar slot 8", Key::Keys(&["8"])),
    action("hotbar_9", "9", "Hotbar slot 9", Key::Keys(&["9"])),
    action("f1", "F1", "F1 Hide HUD", Key::Keys(&["f1"])),
    action("f2", "F2", "F2 Screenshot", Key::Keys(&["f2"])),
    action("f3", "F3", "F3 Debug screen", Key::Keys(&["f3"])),
    action("f3_b", "F3+B", "F3+B Hitboxes", Key::Keys(&["f3", "b"])),
    action("f3_g", "F3+G", "F3+G Chunk borders", Key::Keys(&["f3", "g"])),
    action("f3_h", "F3+H", "F3+H Item tooltips", Key::Keys(&["f3", "h"])),
    action("f3_n", "F3+N", "F3+N Spectator", Key::Keys(&["f3", "n"])),
    action("f3_t", "F3+T", "F3+T Reload packs", Key::Keys(&["f3", "t"])),
    action("f3_f4", "F3+F4", "F3+F4 Game modes", Key::Keys(&["f3", "f4"])),
];

/// The action named `id`.
pub(super) fn find_action(id: &str) -> Option<&'static Action> {
    ACTIONS.iter().find(|a| a.id == id)
}

/// A button the player placed (Edit HUD): its action, its centre as shares of the screen's width
/// and height, and its side as a share of the short side.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Placed {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub size: f32,
}

impl Placed {
    /// Its rectangle in window pixels.
    pub(super) fn rect(&self, [w, h]: [f32; 2]) -> [f32; 4] {
        let s = self.size * w.min(h);
        [self.x * w - s / 2.0, self.y * h - s / 2.0, s, s]
    }
}

/// The file the layout is kept in, in the game folder.
const LAYOUT_FILE: &str = "touch-controls.json";

fn load_layout() -> Option<Vec<Placed>> {
    let text = std::fs::read_to_string(LAYOUT_FILE).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let buttons = value.get("buttons")?.as_array()?;
    Some(
        buttons
            .iter()
            .filter_map(|b| {
                let id = b.get("id")?.as_str()?;
                find_action(id)?;
                let n = |k: &str| b.get(k).and_then(serde_json::Value::as_f64).map(|v| v as f32);
                Some(Placed {
                    id: id.to_owned(),
                    x: n("x")?.clamp(0.0, 1.0),
                    y: n("y")?.clamp(0.0, 1.0),
                    size: n("size")?.clamp(0.04, 0.4),
                })
            })
            .collect(),
    )
}

fn save_layout(buttons: &[Placed]) {
    let list: Vec<serde_json::Value> = buttons
        .iter()
        .map(|b| serde_json::json!({ "id": b.id, "x": b.x, "y": b.y, "size": b.size }))
        .collect();
    let text = serde_json::json!({ "buttons": list }).to_string();
    if let Err(e) = std::fs::write(LAYOUT_FILE, text) {
        log::warn!("Horizon: the touch layout could not be saved: {e}");
    }
}

/// A button: its rectangle in window pixels, what it does and what it shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Pad {
    pub rect: [f32; 4],
    pub key: Key,
    pub label: &'static str,
    /// The action's name (`ACTIONS`).
    pub id: &'static str,
}

/// Where everything sits for a window of `size` pixels whose GUI is drawn at `gui_scale`.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Layout {
    pub keys: Vec<Pad>,
    /// The move stick's area: a finger landing here that hits no button starts the stick.
    pub stick_zone: [f32; 4],
    pub stick_radius: f32,
    /// The slop a finger may move and still be a tap.
    pub slop: f32,
    /// Finger travel per scrolled line in a screen.
    pub line: f32,
    /// The Esc button, top centre: shown in the world and in every screen.
    pub escape: [f32; 4],
}

impl Layout {
    pub(super) fn new(size: [f32; 2], gui_scale: f32) -> Self {
        let [w, h] = size;
        let short = w.min(h);
        // A thumb-sized button: about 9 mm on a phone held sideways.
        let u = short * 0.15;
        let gap = u * 0.2;
        // Clear of the rounded corners, the camera housing and the home indicator.
        let side = (w * 0.055).max(u * 0.5);
        let bottom = u * 0.35;
        let top = u * 0.18;
        let col = |i: f32| w - side - u * (i + 1.0) - gap * i;
        let row = |i: f32| h - bottom - u * (i + 1.0) - gap * i;
        let small = u * 0.72;
        let top_col = |i: f32| w - side - small * (i + 1.0) - gap * i;
        // Over the ends of the hotbar (182 GUI pixels wide, centred), clear of the bars above it
        // (39 GUI pixels from the bottom): beside it they met the buttons on a small phone.
        let hotbar_half = 91.0 * gui_scale;
        let hot_y = h - 39.0 * gui_scale - gap - small;
        let key = |x: f32, y: f32, s: f32, key: Key, label: &'static str| Pad {
            rect: [x, y, s, s],
            key,
            label,
            id: ACTIONS
                .iter()
                .find(|a| a.key == key)
                .map_or("", |a| a.id),
        };
        let keys = vec![
            key(col(0.0), row(0.0), u, Key::Pad(Button::South), "Jump"),
            key(col(1.0), row(0.0), u, Key::Pad(Button::RightStick), "Sneak"),
            key(col(0.0), row(1.0), u, Key::Trigger(0), "Use"),
            key(col(1.0), row(1.0), u, Key::Trigger(1), "Hit"),
            key(top_col(0.0), top, small, Key::Pad(Button::North), "Inv"),
            key(top_col(1.0), top, small, Key::Pad(Button::DPadRight), "Chat"),
            key(top_col(2.0), top, small, Key::Pad(Button::East), "Drop"),
            key(top_col(3.0), top, small, Key::Pad(Button::DPadUp), "Cam"),
            key(
                w / 2.0 - hotbar_half,
                hot_y,
                small,
                Key::Pad(Button::LeftBumper),
                "<",
            ),
            key(
                w / 2.0 + hotbar_half - small,
                hot_y,
                small,
                Key::Pad(Button::RightBumper),
                ">",
            ),
        ];
        Self {
            keys,
            stick_zone: [0.0, h * 0.28, w * 0.40, h * 0.72],
            stick_radius: u * 0.8,
            slop: u * 0.14,
            line: u * 0.35,
            escape: [w / 2.0 - u * 0.4, u * 0.12, u * 0.8, u * 0.5],
        }
    }
    /// The layout with the player's own buttons (Edit HUD) in place of the defaults.
    pub(super) fn with(size: [f32; 2], gui_scale: f32, custom: Option<&[Placed]>) -> Self {
        let mut layout = Self::new(size, gui_scale);
        if let Some(custom) = custom {
            layout.keys = custom
                .iter()
                .filter_map(|p| {
                    let a = find_action(&p.id)?;
                    Some(Pad {
                        rect: p.rect(size),
                        key: a.key,
                        label: a.label,
                        id: a.id,
                    })
                })
                .collect();
        }
        layout
    }

    /// The buttons as Edit HUD keeps them.
    pub(super) fn placed(&self, [w, h]: [f32; 2]) -> Vec<Placed> {
        let short = w.min(h);
        self.keys
            .iter()
            .map(|k| Placed {
                id: k.id.to_owned(),
                x: (k.rect[0] + k.rect[2] / 2.0) / w,
                y: (k.rect[1] + k.rect[3] / 2.0) / h,
                size: k.rect[2] / short,
            })
            .collect()
    }

    fn key_at(&self, at: [f32; 2]) -> Option<usize> {
        self.keys.iter().position(|k| inside(k.rect, at))
    }
}

fn inside([x, y, w, h]: [f32; 4], [px, py]: [f32; 2]) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}

/// What the game should do about a touch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Out {
    /// The pointer is here now (window pixels).
    Cursor([f64; 2]),
    /// A mouse button went down or up.
    Mouse(MouseButton, bool),
    /// Scroll by this many lines (positive moves the content down).
    Wheel(f32),
    /// Turn the view by this much grabbed-mouse motion.
    Look(f64, f64),
    /// The Esc button: what Escape does where the player is (`pad_back`).
    Escape,
    /// Keyboard keys go down (in order) or up (in reverse).
    Keys(&'static [&'static str], bool),
}

impl Out {
    /// The event a mouse would have made; `None` for a turn of the view, which is no event.
    pub(super) fn window_event(self) -> Option<WindowEvent> {
        let device_id = DeviceId::dummy();
        Some(match self {
            Out::Cursor([x, y]) => WindowEvent::CursorMoved {
                device_id,
                position: winit::dpi::PhysicalPosition::new(x, y),
            },
            Out::Mouse(button, down) => WindowEvent::MouseInput {
                device_id,
                state: if down {
                    ElementState::Pressed
                } else {
                    ElementState::Released
                },
                button,
            },
            Out::Wheel(lines) => WindowEvent::MouseWheel {
                device_id,
                delta: MouseScrollDelta::LineDelta(0.0, lines),
                phase: TouchPhase::Moved,
            },
            Out::Look(..) | Out::Escape | Out::Keys(..) => return None,
        })
    }
}

/// A touch in the renderer's pixels. winit scales a touch by the view's content scale and the
/// window's size by the screen's (iOS): where the two differ, a tap landed away from the button
/// drawn under it. Elsewhere a touch is in the window's pixels already.
pub(super) fn touch_pixels(window: &Window, surface: [u32; 2], touch: &Touch) -> [f64; 2] {
    let at = [touch.location.x, touch.location.y];
    if !cfg!(target_os = "ios") {
        return at;
    }
    let outer = window.outer_size();
    let ratio = |s: u32, o: u32| if o == 0 || s == 0 { 1.0 } else { f64::from(s) / f64::from(o) };
    let (fx, fy) = (ratio(surface[0], outer.width), ratio(surface[1], outer.height));
    static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        log::info!(
            "Horizon iOS: touches scaled by {fx:.3} x {fy:.3} (view scale {}, window {}x{}, \
             frame {}x{}); the first at {:.0}, {:.0}",
            window.scale_factor(),
            outer.width,
            outer.height,
            surface[0],
            surface[1],
            at[0],
            at[1],
        );
    }
    [at[0] * fx, at[1] * fy]
}

/// The layout for a window of `size` pixels, at the GUI scale the HUD draws it with.
pub(super) fn layout_for(size: [u32; 2]) -> Layout {
    custom_layout_for(size, None)
}

fn custom_layout_for(size: [u32; 2], custom: Option<&[Placed]>) -> Layout {
    let scale = HudBatch::new(size).scale;
    Layout::with([size[0] as f32, size[1] as f32], scale, custom)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Role {
    /// A screen's pointer, before it is known to be a tap, a scroll or a drag.
    Pending,
    Scroll,
    Drag,
    /// A second finger in a screen: a right click when it lifts.
    Second,
    Stick,
    Look,
    Key(usize),
    /// On the Esc button: nothing more until it lifts.
    Escape,
}

#[derive(Clone, Copy, Debug)]
struct Finger {
    id: u64,
    role: Role,
    start: [f64; 2],
    last: [f64; 2],
    began: Instant,
}

/// The fingers down and what they hold.
#[derive(Default)]
pub(super) struct Controls {
    fingers: Vec<Finger>,
    /// The fingers were last read in the world (true) or in a screen.
    world: bool,
    /// What the fingers hold now.
    held: PadState,
    /// Buttons that went down since the last poll: a tap shorter than a frame still counts.
    pulse: PadState,
    /// A finger has touched the glass in this run.
    pub(super) used: bool,
    /// iOS: the system keyboard was put away (its return key, or a tap off any text field)
    /// while a field keeps the focus, as the game's fields do.
    pub(super) keyboard_hidden: bool,
    /// A text field had the focus at the last `sync_ime`.
    pub(super) field_focused: bool,
    /// The player's own layout (Edit HUD), once `touch-controls.json` was read.
    custom: Option<Vec<Placed>>,
    loaded: bool,
    /// Edit HUD, while it is open.
    editor: Option<super::touch_editor::Editor>,
}

impl Controls {
    /// Whether touches are read at all: always on iOS; `HORIZON_TOUCH=1` elsewhere (a
    /// touchscreen laptop's fingers already arrive as a mouse).
    pub(super) fn enabled() -> bool {
        cfg!(target_os = "ios") || std::env::var_os("HORIZON_TOUCH").is_some_and(|v| v == "1")
    }

    /// Reads `touch-controls.json` once.
    fn load(&mut self) {
        if !self.loaded {
            self.loaded = true;
            self.custom = load_layout();
        }
    }

    /// The virtual pad for this poll.
    pub(super) fn poll(&mut self) -> PadState {
        let state = self.held.merge(&std::mem::take(&mut self.pulse));
        // `merge` adds sticks: the pulse carries none.
        PadState {
            left: self.held.left,
            right: [0.0, 0.0],
            ..state
        }
    }

    /// The move stick's base and knob, in window pixels, while a finger holds it.
    pub(super) fn stick(&self) -> Option<([f64; 2], [f64; 2])> {
        self.fingers
            .iter()
            .find(|f| f.role == Role::Stick)
            .map(|f| (f.start, f.last))
    }

    /// Whether a finger is on the Esc button.
    pub(super) fn escape_held(&self) -> bool {
        self.fingers.iter().any(|f| f.role == Role::Escape)
    }

    /// Whether the button at `index` of the layout is held.
    pub(super) fn key_held(&self, index: usize) -> bool {
        self.fingers.iter().any(|f| f.role == Role::Key(index))
    }

    /// Lets go of everything: the mode changed under the fingers (a screen opened or closed).
    fn release(&mut self, layout: &Layout, out: &mut Vec<Out>) {
        for f in self.fingers.drain(..) {
            match f.role {
                Role::Drag => out.push(Out::Mouse(MouseButton::Left, false)),
                Role::Key(i) => {
                    if let Some(Key::Keys(keys)) = layout.keys.get(i).map(|k| k.key) {
                        out.push(Out::Keys(keys, false));
                    }
                }
                _ => {}
            }
        }
        self.held = PadState::default();
    }

    /// One touch event. `world` is whether the game holds the pointer (the player is in the
    /// world with no screen open).
    pub(super) fn touch(
        &mut self,
        id: u64,
        phase: TouchPhase,
        at: [f64; 2],
        now: Instant,
        world: bool,
        layout: &Layout,
    ) -> Vec<Out> {
        let mut out = Vec::new();
        self.used = true;
        if world != self.world {
            self.release(layout, &mut out);
            self.world = world;
        }
        let known = self.fingers.iter().position(|f| f.id == id);
        match (phase, known) {
            (TouchPhase::Started, _) => {
                // A finger the system never ended.
                if let Some(i) = known {
                    self.lift(i, at, false, layout, &mut out);
                }
                let role = if inside(layout.escape, [at[0] as f32, at[1] as f32]) {
                    out.push(Out::Escape);
                    Role::Escape
                } else if world {
                    self.world_role(at, layout)
                } else if self.fingers.is_empty() {
                    out.push(Out::Cursor(at));
                    Role::Pending
                } else {
                    Role::Second
                };
                self.fingers.push(Finger {
                    id,
                    role,
                    start: at,
                    last: at,
                    began: now,
                });
                if let Role::Key(i) = role {
                    match layout.keys[i].key {
                        Key::Keys(keys) => out.push(Out::Keys(keys, true)),
                        key => self.press(key),
                    }
                }
            }
            (TouchPhase::Moved, Some(i)) => self.moved(i, at, now, layout, &mut out),
            (TouchPhase::Ended, Some(i)) => self.lift(i, at, true, layout, &mut out),
            (TouchPhase::Cancelled, Some(i)) => self.lift(i, at, false, layout, &mut out),
            _ => {}
        }
        self.hold(layout);
        out
    }

    fn world_role(&self, at: [f64; 2], layout: &Layout) -> Role {
        let p = [at[0] as f32, at[1] as f32];
        if let Some(i) = layout.key_at(p) {
            return Role::Key(i);
        }
        let stick_free = !self.fingers.iter().any(|f| f.role == Role::Stick);
        if stick_free && inside(layout.stick_zone, p) {
            return Role::Stick;
        }
        Role::Look
    }

    fn press(&mut self, key: Key) {
        match key {
            Key::Pad(button) => self.pulse.set(button, true),
            Key::Trigger(i) => self.pulse.triggers[i] = 1.0,
            Key::Keys(_) => {}
        }
    }

    fn moved(&mut self, i: usize, at: [f64; 2], now: Instant, layout: &Layout, out: &mut Vec<Out>) {
        let f = self.fingers[i];
        let (dx, dy) = (at[0] - f.last[0], at[1] - f.last[1]);
        self.fingers[i].last = at;
        match f.role {
            Role::Pending => {
                let (tx, ty) = (at[0] - f.start[0], at[1] - f.start[1]);
                if tx.hypot(ty) <= f64::from(layout.slop) {
                    return;
                }
                if tx.abs() > ty.abs() || now.duration_since(f.began) >= HOLD {
                    self.fingers[i].role = Role::Drag;
                    out.push(Out::Cursor(f.start));
                    out.push(Out::Mouse(MouseButton::Left, true));
                    out.push(Out::Cursor(at));
                } else {
                    self.fingers[i].role = Role::Scroll;
                    out.push(Out::Wheel(ty as f32 / layout.line));
                }
            }
            Role::Scroll => out.push(Out::Wheel(dy as f32 / layout.line)),
            Role::Drag => out.push(Out::Cursor(at)),
            Role::Look | Role::Key(_) => {
                if dx != 0.0 || dy != 0.0 {
                    out.push(Out::Look(dx * LOOK_SCALE, dy * LOOK_SCALE));
                }
            }
            Role::Stick | Role::Second | Role::Escape => {}
        }
    }

    fn lift(&mut self, i: usize, at: [f64; 2], ended: bool, layout: &Layout, out: &mut Vec<Out>) {
        let f = self.fingers.remove(i);
        match f.role {
            Role::Pending if ended => {
                out.push(Out::Cursor(at));
                out.push(Out::Mouse(MouseButton::Left, true));
                out.push(Out::Mouse(MouseButton::Left, false));
            }
            Role::Drag => {
                out.push(Out::Cursor(at));
                out.push(Out::Mouse(MouseButton::Left, false));
            }
            // A right click where the first finger rests; that finger is no longer a tap.
            Role::Second if ended => {
                if let Some(first) = self.fingers.iter_mut().find(|f| f.role == Role::Pending) {
                    first.role = Role::Scroll;
                    out.push(Out::Cursor(first.last));
                    out.push(Out::Mouse(MouseButton::Right, true));
                    out.push(Out::Mouse(MouseButton::Right, false));
                }
            }
            Role::Key(i) => {
                if let Some(Key::Keys(keys)) = layout.keys.get(i).map(|k| k.key) {
                    out.push(Out::Keys(keys, false));
                }
            }
            _ => {}
        }
    }

    /// What the fingers hold, from scratch.
    fn hold(&mut self, layout: &Layout) {
        let mut held = PadState::default();
        for f in &self.fingers {
            match f.role {
                Role::Key(i) => match layout.keys.get(i).map(|k| k.key) {
                    Some(Key::Pad(button)) => held.set(button, true),
                    Some(Key::Trigger(t)) => held.triggers[t] = 1.0,
                    Some(Key::Keys(_)) | None => {}
                },
                Role::Stick => {
                    let r = f64::from(layout.stick_radius);
                    let x = ((f.last[0] - f.start[0]) / r) as f32;
                    // Up is positive on a pad, down on the glass.
                    let y = ((f.start[1] - f.last[1]) / r) as f32;
                    let length = x.hypot(y);
                    if length > SPRINT_AT {
                        held.set(Button::LeftStick, true);
                    }
                    let shrink = if length > 1.0 { 1.0 / length } else { 1.0 };
                    held.left = [x * shrink, y * shrink];
                }
                _ => {}
            }
        }
        self.held = held;
    }
}

impl Game<'_> {
    /// Whether the fingers play the world (the game holds the pointer) or work a screen.
    fn touch_world(&self) -> bool {
        self.captured && self.title.is_none() && self.screen == Screen::Playing
    }

    fn touch_size(&self) -> Option<[u32; 2]> {
        let window = self.window.as_ref()?;
        Some(self.renderer.as_ref().map_or_else(
            || window.inner_size().into(),
            |r| [r.config.width, r.config.height],
        ))
    }

    fn touch_layout(&self) -> Option<Layout> {
        Some(custom_layout_for(self.touch_size()?, self.touch.custom.as_deref()))
    }

    /// Edit HUD (the title's button in Quit Game's place on iOS).
    pub(super) fn open_touch_editor(&mut self) {
        self.touch.load();
        let Some(size) = self.touch_size() else {
            return;
        };
        let fsize = [size[0] as f32, size[1] as f32];
        let buttons = match &self.touch.custom {
            Some(custom) => custom.clone(),
            None => layout_for(size).placed(fsize),
        };
        self.touch.editor = Some(super::touch_editor::Editor::new(buttons));
    }

    /// A finger in Edit HUD.
    fn touch_editor_event(&mut self, touch: &Touch, at: [f64; 2], size: [u32; 2]) {
        let fsize = [size[0] as f32, size[1] as f32];
        let defaults = layout_for(size).placed(fsize);
        let Some(editor) = &mut self.touch.editor else {
            return;
        };
        let at = [at[0] as f32, at[1] as f32];
        if let Some(done) = editor.touch(touch.id, touch.phase, at, fsize, &defaults) {
            save_layout(&done);
            // The default layout stays the default: it follows the game's later changes.
            self.touch.custom = (done != defaults).then_some(done);
            if self.touch.custom.is_none() {
                let _ = std::fs::remove_file(LAYOUT_FILE);
            }
            self.touch.editor = None;
        }
    }

    /// A finger on the glass (`WindowEvent::Touch`).
    pub(super) fn touch_event(&mut self, event_loop: &dyn EventLoopCtl, id: WindowId, touch: Touch) {
        if !Controls::enabled() {
            return;
        }
        self.touch.load();
        let Some(layout) = self.touch_layout() else {
            return;
        };
        let Some(window) = self.window.clone() else {
            return;
        };
        let size = self.renderer.as_ref().map_or_else(
            || window.inner_size().into(),
            |r| [r.config.width, r.config.height],
        );
        let at = touch_pixels(&window, size, &touch);
        if self.touch.editor.is_some() {
            self.last_input = Instant::now();
            return self.touch_editor_event(&touch, at, size);
        }
        let world = self.touch_world();
        // The system keyboard covers half the screen and the game's fields keep the focus: a tap
        // on a text field brings it up, a tap anywhere else puts it away.
        if !world && touch.phase == TouchPhase::Started {
            let scale = HudBatch::new(size).scale;
            let gui = (at[0] as f32 / scale, at[1] as f32 / scale);
            self.touch.keyboard_hidden =
                !super::pad_menu::field_at(&self.pad_visible_targets(), gui);
        }
        let outs = self
            .touch
            .touch(touch.id, touch.phase, at, Instant::now(), world, &layout);
        self.last_input = Instant::now();
        for out in outs {
            match (out, out.window_event()) {
                (_, Some(event)) => {
                    self.handle_window_event(event_loop, id, event, input_trace::InputOrigin::User);
                }
                // MouseHandler.onMove with the pointer grabbed, as `device_event` does.
                (Out::Look(x, y), None) => {
                    if self.pending_save.is_none() && self.touch_world() {
                        self.mouse.add_motion(x, y);
                    }
                }
                (Out::Escape, None) => {
                    if let Err(e) = self.pad_back() {
                        log::warn!("Horizon: Esc: {e}");
                    }
                }
                // The control socket's own key path: F3 combinations, F2's screenshot and the
                // rest behave as on a keyboard.
                (Out::Keys(keys, down), None) => {
                    let names: Vec<&str> = if down {
                        keys.to_vec()
                    } else {
                        keys.iter().rev().copied().collect()
                    };
                    for name in names {
                        let action = if down { "down" } else { "up" };
                        self.replay_line(&format!("key {name} {action}"), event_loop);
                    }
                }
                _ => {}
            }
        }
    }

    /// iOS: the system keyboard's return key, which types a line break the game's fields don't
    /// take: Enter, as the keyboard's would be (it sends a chat line, runs a command, confirms a
    /// sign or a screen), and the keyboard goes away. Over the title, with no world, the screens
    /// have their Done buttons: the keyboard goes away only.
    pub(super) fn keyboard_return(&mut self, event_loop: &dyn EventLoopCtl) {
        self.touch.keyboard_hidden = true;
        if self.title.is_none() && self.player.is_some() {
            self.replay_line("key enter tap", event_loop);
        }
        self.sync_ime();
    }

    /// iOS: the app stopped being the active one (the app switcher, a call, the lock screen) or
    /// is active again. The window's focus says so to the game: it pauses, which saves the
    /// world, and lets go of what was held.
    pub(super) fn app_active(&mut self, event_loop: &dyn EventLoopCtl, active: bool) {
        let Some(id) = self.window.as_ref().map(|w| w.id()) else {
            return;
        };
        if self.focused != active {
            self.handle_window_event(
                event_loop,
                id,
                WindowEvent::Focused(active),
                input_trace::InputOrigin::User,
            );
        }
        if !active {
            // Now, not after the half second a desktop window waits: the app may be off the
            // screen and stopped before then, and the pause is what saves the world.
            let now = Instant::now();
            self.focus_lost = Some(now.checked_sub(Duration::from_secs(1)).unwrap_or(now));
            self.pause_if_inactive();
        }
    }

    /// Edit HUD over the title: the buttons where they would be in the world, the move stick's
    /// area, the tools and the Add list.
    fn draw_touch_editor(&self, batch: &mut HudBatch, layout: &Layout) {
        let Some(editor) = &self.touch.editor else {
            return;
        };
        let Some(size) = self.touch_size() else {
            return;
        };
        let fsize = [size[0] as f32, size[1] as f32];
        let s = batch.scale;
        let atlas = &self.gui.atlas;
        let white = rgba(255, 255, 255, 230);
        let plate = |batch: &mut HudBatch, r: [f32; 4], fill: u32, label: &str| {
            let [x, y, w, h] = r.map(|v| v / s);
            batch.fill(atlas, x - 1.0, y - 1.0, w + 2.0, h + 2.0, rgba(0, 0, 0, 120));
            batch.fill(atlas, x, y, w, h, fill);
            let tw = HudBatch::text_width(atlas, label);
            batch.text(atlas, label, x + (w - tw) / 2.0, y + (h - 8.0) / 2.0, white, true);
        };
        batch.fill(atlas, 0.0, 0.0, fsize[0] / s, fsize[1] / s, rgba(0, 0, 0, 150));
        let [zx, zy, zw, zh] = layout.stick_zone.map(|v| v / s);
        batch.fill(atlas, zx, zy, zw, zh, rgba(255, 255, 255, 14));
        let note = "Move stick area";
        batch.text(atlas, note, zx + 6.0, zy + 6.0, rgba(255, 255, 255, 120), true);
        for (i, b) in editor.buttons.iter().enumerate() {
            let label = find_action(&b.id).map_or("?", |a| a.label);
            let fill = if editor.selected == Some(i) {
                rgba(90, 160, 255, 170)
            } else {
                rgba(255, 255, 255, 50)
            };
            plate(batch, b.rect(fsize), fill, label);
        }
        for (tool, label, r) in super::touch_editor::toolbar(fsize) {
            let needs_selection = matches!(
                tool,
                super::touch_editor::Tool::Remove
                    | super::touch_editor::Tool::Smaller
                    | super::touch_editor::Tool::Bigger
            );
            let fill = if tool == super::touch_editor::Tool::Done {
                rgba(60, 150, 70, 200)
            } else if needs_selection && editor.selected.is_none() {
                rgba(40, 40, 40, 160)
            } else {
                rgba(70, 70, 70, 210)
            };
            plate(batch, r, fill, label);
        }
        let hint = if editor.picking {
            "Tap a button to add it"
        } else if editor.selected.is_some() {
            "Drag to move it; Remove, Smaller and Bigger change it"
        } else {
            "Drag a button to move it, or tap one to select it"
        };
        let u = fsize[0].min(fsize[1]) * 0.15;
        let tw = HudBatch::text_width(atlas, hint);
        batch.text(atlas, hint, (fsize[0] / s - tw) / 2.0, (u * 0.75) / s, white, true);
        if editor.picking {
            let (panel, tiles) = super::touch_editor::picker(fsize, &editor.buttons);
            let [px, py, pw, ph] = panel.map(|v| v / s);
            batch.fill(atlas, px - 1.0, py - 1.0, pw + 2.0, ph + 2.0, rgba(255, 255, 255, 90));
            batch.fill(atlas, px, py, pw, ph, rgba(20, 20, 20, 235));
            let title = "Add a button";
            let tw = HudBatch::text_width(atlas, title);
            batch.text(atlas, title, px + (pw - tw) / 2.0, py + 6.0, white, true);
            for (action, r) in tiles {
                plate(batch, r, rgba(70, 70, 70, 230), ACTIONS[action].name);
            }
        }
    }

    /// The buttons and the stick over the HUD, while the fingers play the world.
    pub(super) fn draw_touch_controls(&mut self, batch: &mut HudBatch) {
        if !Controls::enabled() {
            return;
        }
        self.touch.load();
        let Some(layout) = self.touch_layout() else {
            return;
        };
        if self.touch.editor.is_some() {
            return self.draw_touch_editor(batch, &layout);
        }
        let s = batch.scale;
        let atlas = &self.gui.atlas;
        let idle = rgba(255, 255, 255, 34);
        let down = rgba(255, 255, 255, 96);
        let edge = rgba(0, 0, 0, 70);
        {
            let [x, y, w, h] = layout.escape.map(|v| v / s);
            batch.fill(atlas, x - 1.0, y - 1.0, w + 2.0, h + 2.0, edge);
            let plate = if self.touch.escape_held() {
                down
            } else {
                rgba(0, 0, 0, 90)
            };
            batch.fill(atlas, x, y, w, h, plate);
            let tw = HudBatch::text_width(atlas, "Esc");
            batch.text(
                atlas,
                "Esc",
                x + (w - tw) / 2.0,
                y + (h - 8.0) / 2.0,
                rgba(255, 255, 255, 220),
                true,
            );
        }
        if !self.touch_world() || self.ui.hide_gui {
            return;
        }
        for (i, key) in layout.keys.iter().enumerate() {
            let [x, y, w, h] = key.rect.map(|v| v / s);
            batch.fill(atlas, x - 1.0, y - 1.0, w + 2.0, h + 2.0, edge);
            let colour = if self.touch.key_held(i) { down } else { idle };
            batch.fill(atlas, x, y, w, h, colour);
            let tw = HudBatch::text_width(atlas, key.label);
            batch.text(
                atlas,
                key.label,
                x + (w - tw) / 2.0,
                y + (h - 8.0) / 2.0,
                rgba(255, 255, 255, 200),
                true,
            );
        }
        if let Some((base, knob)) = self.touch.stick() {
            let r = layout.stick_radius / s;
            let [bx, by] = base.map(|v| v as f32 / s);
            batch.fill(atlas, bx - r - 1.0, by - r - 1.0, r * 2.0 + 2.0, r * 2.0 + 2.0, edge);
            batch.fill(atlas, bx - r, by - r, r * 2.0, r * 2.0, idle);
            let (dx, dy) = (knob[0] as f32 / s - bx, knob[1] as f32 / s - by);
            let length = dx.hypot(dy).max(0.001);
            let reach = length.min(r) / length;
            let k = r * 0.45;
            batch.fill(
                atlas,
                bx + dx * reach - k,
                by + dy * reach - k,
                k * 2.0,
                k * 2.0,
                down,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: [f32; 2] = [2556.0, 1179.0];

    fn layout() -> Layout {
        Layout::new(SIZE, 4.0)
    }
    fn centre(l: &Layout, label: &str) -> [f64; 2] {
        let k = l.keys.iter().find(|k| k.label == label).unwrap();
        [
            f64::from(k.rect[0] + k.rect[2] / 2.0),
            f64::from(k.rect[1] + k.rect[3] / 2.0),
        ]
    }

    #[test]
    fn the_buttons_fit_the_screen_and_do_not_overlap() {
        for size in [SIZE, [1334.0, 750.0], [2796.0, 1290.0], [2048.0, 1536.0]] {
            let l = Layout::new(size, 3.0);
            for (i, a) in l.keys.iter().enumerate() {
                let [x, y, w, h] = a.rect;
                assert!(x >= 0.0 && y >= 0.0 && x + w <= size[0] && y + h <= size[1], "{a:?}");
                for b in &l.keys[i + 1..] {
                    let [bx, by, bw, bh] = b.rect;
                    let apart = x + w <= bx || bx + bw <= x || y + h <= by || by + bh <= y;
                    assert!(apart, "{a:?} overlaps {b:?} at {size:?}");
                }
                let [ex, ey, ew, eh] = l.escape;
                let clear = x + w <= ex || ex + ew <= x || y + h <= ey || ey + eh <= y;
                assert!(clear, "{a:?} overlaps Esc at {size:?}");
                // The stick's area holds no button but the hotbar's, which a finger meets first.
                assert!(!inside(l.stick_zone, [x + w / 2.0, y + h / 2.0]) || a.label == "<");
            }
        }
    }

    #[test]
    fn esc_works_in_the_world_and_in_screens_and_does_nothing_else() {
        let (l, t0) = (layout(), Instant::now());
        let [x, y, w, h] = l.escape;
        let esc = [f64::from(x + w / 2.0), f64::from(y + h / 2.0)];
        for world in [true, false] {
            let mut c = Controls::default();
            assert_eq!(c.touch(1, TouchPhase::Started, esc, t0, world, &l), [Out::Escape]);
            assert!(c.escape_held());
            let away = [esc[0] + 200.0, esc[1] + 200.0];
            assert!(c.touch(1, TouchPhase::Moved, away, t0, world, &l).is_empty());
            assert!(c.touch(1, TouchPhase::Ended, away, t0, world, &l).is_empty());
            assert_eq!(c.poll(), PadState::default());
        }
    }

    #[test]
    fn a_tap_in_a_screen_is_a_click_where_it_lands() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        assert_eq!(
            c.touch(1, TouchPhase::Started, [300.0, 200.0], t0, false, &l),
            [Out::Cursor([300.0, 200.0])]
        );
        assert_eq!(
            c.touch(1, TouchPhase::Ended, [302.0, 201.0], t0, false, &l),
            [
                Out::Cursor([302.0, 201.0]),
                Out::Mouse(MouseButton::Left, true),
                Out::Mouse(MouseButton::Left, false)
            ]
        );
    }

    #[test]
    fn a_quick_vertical_swipe_scrolls_and_never_clicks() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        c.touch(1, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        let moved = c.touch(1, TouchPhase::Moved, [305.0, 600.0 + f64::from(l.line) * 2.0], t0, false, &l);
        assert_eq!(moved, [Out::Wheel(2.0)]);
        assert!(c.touch(1, TouchPhase::Ended, [305.0, 900.0], t0, false, &l).is_empty());
    }

    #[test]
    fn a_sideways_or_held_drag_is_a_button_drag() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        c.touch(1, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        assert_eq!(
            c.touch(1, TouchPhase::Moved, [500.0, 610.0], t0, false, &l),
            [
                Out::Cursor([300.0, 600.0]),
                Out::Mouse(MouseButton::Left, true),
                Out::Cursor([500.0, 610.0])
            ]
        );
        assert_eq!(
            c.touch(1, TouchPhase::Ended, [520.0, 610.0], t0, false, &l),
            [Out::Cursor([520.0, 610.0]), Out::Mouse(MouseButton::Left, false)]
        );
        // Held still first, then moved down: a drag too.
        c.touch(2, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        let later = t0 + HOLD;
        let moved = c.touch(2, TouchPhase::Moved, [300.0, 800.0], later, false, &l);
        assert_eq!(moved[1], Out::Mouse(MouseButton::Left, true));
    }

    #[test]
    fn a_second_finger_right_clicks_at_the_first() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        c.touch(1, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        assert!(c.touch(2, TouchPhase::Started, [900.0, 600.0], t0, false, &l).is_empty());
        assert_eq!(
            c.touch(2, TouchPhase::Ended, [900.0, 600.0], t0, false, &l),
            [
                Out::Cursor([300.0, 600.0]),
                Out::Mouse(MouseButton::Right, true),
                Out::Mouse(MouseButton::Right, false)
            ]
        );
        // The first finger lifting afterwards does not left click.
        assert!(c.touch(1, TouchPhase::Ended, [300.0, 600.0], t0, false, &l).is_empty());
    }

    #[test]
    fn the_stick_moves_and_sprints_past_its_edge() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        let base = [400.0, 900.0];
        c.touch(1, TouchPhase::Started, base, t0, true, &l);
        let r = f64::from(l.stick_radius);
        c.touch(1, TouchPhase::Moved, [base[0], base[1] - r / 2.0], t0, true, &l);
        let pad = c.poll();
        assert!((pad.left[1] - 0.5).abs() < 1e-4 && pad.left[0] == 0.0, "{pad:?}");
        assert!(!pad.held(Button::LeftStick));
        c.touch(1, TouchPhase::Moved, [base[0], base[1] - r * 2.0], t0, true, &l);
        let pad = c.poll();
        assert!((pad.left[1] - 1.0).abs() < 1e-4);
        assert!(pad.held(Button::LeftStick));
        c.touch(1, TouchPhase::Ended, base, t0, true, &l);
        assert_eq!(c.poll(), PadState::default());
    }

    #[test]
    fn a_button_tapped_between_two_polls_still_counts_once() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        let jump = centre(&l, "Jump");
        c.touch(1, TouchPhase::Started, jump, t0, true, &l);
        c.touch(1, TouchPhase::Ended, jump, t0, true, &l);
        assert!(c.poll().held(Button::South));
        assert!(!c.poll().held(Button::South));
        // The triggers too.
        let hit = centre(&l, "Hit");
        c.touch(1, TouchPhase::Started, hit, t0, true, &l);
        assert_eq!(c.poll().triggers, [0.0, 1.0]);
        assert_eq!(c.poll().triggers, [0.0, 1.0]);
        c.touch(1, TouchPhase::Ended, hit, t0, true, &l);
        assert_eq!(c.poll().triggers, [0.0, 0.0]);
    }

    #[test]
    fn fingers_off_the_stick_and_on_buttons_turn_the_view() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        c.touch(1, TouchPhase::Started, [1500.0, 500.0], t0, true, &l);
        assert_eq!(
            c.touch(1, TouchPhase::Moved, [1510.0, 480.0], t0, true, &l),
            [Out::Look(10.0 * LOOK_SCALE, -20.0 * LOOK_SCALE)]
        );
        let hit = centre(&l, "Hit");
        c.touch(2, TouchPhase::Started, hit, t0, true, &l);
        let moved = c.touch(2, TouchPhase::Moved, [hit[0] + 4.0, hit[1]], t0, true, &l);
        assert_eq!(moved, [Out::Look(4.0 * LOOK_SCALE, 0.0)]);
        // Still held after sliding off the button.
        assert_eq!(c.poll().triggers[1], 1.0);
    }

    #[test]
    fn a_screen_opening_lets_go_of_everything() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        c.touch(1, TouchPhase::Started, [400.0, 900.0], t0, true, &l);
        c.touch(1, TouchPhase::Moved, [400.0, 700.0], t0, true, &l);
        c.poll();
        // The inventory opened: the next touch is a screen's.
        let out = c.touch(2, TouchPhase::Started, [800.0, 400.0], t0, false, &l);
        assert_eq!(out, [Out::Cursor([800.0, 400.0])]);
        assert_eq!(c.poll(), PadState::default());
        // The old stick finger lifting does nothing.
        assert!(c.touch(1, TouchPhase::Ended, [400.0, 700.0], t0, false, &l).is_empty());
    }
}
