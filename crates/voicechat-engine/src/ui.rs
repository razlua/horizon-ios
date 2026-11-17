//! Ferrite VC's menu (V): three tabs on the mod's own screen. Voice has the microphone, the
//! activation and the volumes (sliders) with a live microphone level; Groups lists the server's
//! groups and makes new ones; Keys shows the hotkeys as bound now. It is drawn with Minecraft's
//! own widget sprites (buttons, sliders, text fields), 20 px tall like the game's, so it reads
//! as part of the game and is easy to tap. It lays itself out from a [`View`] of the voice chat
//! each frame and keeps only the tab, what is typed and what is focused.
use crate::Settings;
use crate::net::Mode;

/// What a hook draws, in GUI pixels: a filled `[x, y, w, h]` in ARGB, a line of text at its top
/// left in RGB (with or without a shadow), or one of the game's GUI sprites
/// (`gui/sprites/widget/button`) stretched over a rect and tinted (ARGB).
#[derive(Clone, Debug, PartialEq)]
pub enum Draw {
    Fill([f32; 4], u32),
    Text(String, [f32; 2], u32, bool),
    Sprite(String, [f32; 4], u32),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Mute,
    Disable,
    Mode,
    Labels,
    Volume(i32),
    Gain(i32),
    Threshold(i32),
    /// A slider, set where it was tapped (`Menu::fraction`).
    Slide(Slider),
    Join(u128),
    JoinWithPassword,
    CancelJoin,
    Leave,
    Focus(Field),
    Kind,
    Create,
    Scroll(i32),
    Tab(Tab),
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name,
    Password,
    JoinPassword,
}

/// The menu's tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Voice,
    Groups,
    Keys,
}

/// The Voice tab's sliders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slider {
    Volume,
    Gain,
    Threshold,
}

/// A group as the menu lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupRow {
    pub id: u128,
    pub name: String,
    pub has_password: bool,
    pub members: Vec<String>,
    pub ours: bool,
}

/// The voice chat as the menu shows it this frame.
pub struct View {
    pub status: (String, u32),
    pub settings: Settings,
    pub level_db: f32,
    pub connected: bool,
    pub groups_enabled: bool,
    pub groups: Vec<GroupRow>,
    /// The hotkeys: what each does and the key bound to it, already named.
    pub hotkeys: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Widget {
    pub rect: [f32; 4],
    pub action: Action,
}

/// The menu's name, in its header.
pub const TITLE: &str = "Ferrite VC";

// Colours: ARGB for fills and sprites, RGB for text.
const SHADE: u32 = 0xA000_0000;
const PANEL: u32 = 0xF212_1218;
const PANEL_EDGE: u32 = 0xFF2E_2E38;
const HEADER: u32 = 0xFF1A_1A22;
const WELL: u32 = 0xFF0A_0A0E;
const ACCENT: u32 = 0xFFFF_C21A;
const WHITE: u32 = 0xFFFF_FFFF;
const ON: u32 = 0xFF6C_E06C;
const OFF: u32 = 0xFFFF_6B6B;
const ROW_FILL: u32 = 0x5028_2834;
const ROW_OURS: u32 = 0x502A_4A2A;
const INK: u32 = 0xFF_FFFF;
const GREY: u32 = 0xA8_A8B0;
const DARK: u32 = 0x6E_6E78;
const GOLD: u32 = 0xFF_C21A;
const GREEN: u32 = 0x6C_E06C;
const RED: u32 = 0xFF_6B6B;

const BUTTON: &str = "gui/sprites/widget/button";
const BUTTON_HOVER: &str = "gui/sprites/widget/button_highlighted";
const SLIDER: &str = "gui/sprites/widget/slider";
const SLIDER_HOVER: &str = "gui/sprites/widget/slider_highlighted";
const HANDLE: &str = "gui/sprites/widget/slider_handle";
const HANDLE_HOVER: &str = "gui/sprites/widget/slider_handle_highlighted";
const FIELD: &str = "gui/sprites/widget/text_field";
const FIELD_FOCUS: &str = "gui/sprites/widget/text_field_highlighted";

pub const NAME_MAX: usize = 24;
const PASSWORD_MAX: usize = 24;
/// A widget's height, as the game's buttons.
const H: f32 = 20.0;
/// A group's row in the list.
const ROW: f32 = 26.0;

#[derive(Default)]
pub struct Menu {
    pub open: bool,
    close_requested: bool,
    pub tab: Tab,
    pub name: String,
    pub password: String,
    pub kind: i16,
    pub focus: Option<Field>,
    /// The password group being joined, and what is typed for it.
    pub joining: Option<u128>,
    pub join_password: String,
    pub message: Option<(String, u32)>,
    scroll: usize,
    /// The character of the key that opened the menu, typed with it.
    skip_char: Option<char>,
    widgets: Vec<Widget>,
}

struct Out<'a> {
    draws: Vec<Draw>,
    widgets: Vec<Widget>,
    mouse: [f32; 2],
    measure: &'a dyn Fn(&str) -> f32,
}

fn inside(rect: [f32; 4], at: [f32; 2]) -> bool {
    at[0] >= rect[0] && at[0] < rect[0] + rect[2] && at[1] >= rect[1] && at[1] < rect[1] + rect[3]
}

impl Out<'_> {
    fn fill(&mut self, rect: [f32; 4], argb: u32) {
        self.draws.push(Draw::Fill(rect, argb));
    }
    fn sprite(&mut self, name: &str, rect: [f32; 4]) {
        self.draws.push(Draw::Sprite(name.to_owned(), rect, WHITE));
    }
    fn text(&mut self, text: &str, at: [f32; 2], rgb: u32) {
        self.draws.push(Draw::Text(text.to_owned(), at, rgb, true));
    }
    fn width(&self, text: &str) -> f32 {
        (self.measure)(text)
    }
    /// `text` centred in `rect`, cut to fit.
    fn centred(&mut self, text: &str, rect: [f32; 4], rgb: u32) {
        let text = self.fit(text, rect[2] - 6.0);
        let w = self.width(&text);
        self.text(&text, [rect[0] + (rect[2] - w) / 2.0, rect[1] + (rect[3] - 8.0) / 2.0], rgb);
    }
    /// `text` cut to `max` pixels with an ellipsis.
    fn fit(&self, text: &str, max: f32) -> String {
        if self.width(text) <= max {
            return text.to_owned();
        }
        let mut cut = text.to_owned();
        while !cut.is_empty() && self.width(&format!("{cut}...")) > max {
            cut.pop();
        }
        format!("{cut}...")
    }
    /// One of the game's buttons. `state` puts a green (on) or red (off) light at its left.
    fn button(&mut self, rect: [f32; 4], label: &str, action: Action, state: Option<bool>) {
        let hover = inside(rect, self.mouse);
        self.sprite(if hover { BUTTON_HOVER } else { BUTTON }, rect);
        let [x, y, w, h] = rect;
        match state {
            Some(on) => {
                self.fill([x + 6.0, y + (h - 6.0) / 2.0, 6.0, 6.0], 0xFF00_0000);
                self.fill([x + 7.0, y + (h - 4.0) / 2.0, 4.0, 4.0], if on { ON } else { OFF });
                self.centred(label, [x + 12.0, y, w - 12.0, h], INK);
            }
            None => self.centred(label, rect, INK),
        }
        self.widgets.push(Widget { rect, action });
    }
    /// A tab: a button, lit and underlined in Ferrite's yellow when it is the open one.
    fn tab(&mut self, rect: [f32; 4], label: &str, action: Action, selected: bool) {
        let hover = inside(rect, self.mouse);
        self.sprite(if hover || selected { BUTTON_HOVER } else { BUTTON }, rect);
        let [x, y, w, h] = rect;
        self.centred(label, rect, if selected { GOLD } else { GREY });
        if selected {
            self.fill([x + 4.0, y + h - 3.0, w - 8.0, 2.0], ACCENT);
        }
        self.widgets.push(Widget { rect, action });
    }
    /// The game's slider: the value's label over it, the handle at `fraction`.
    fn slider(&mut self, rect: [f32; 4], label: &str, fraction: f32, action: Action) {
        let hover = inside(rect, self.mouse);
        self.sprite(if hover { SLIDER_HOVER } else { SLIDER }, rect);
        let [x, y, w, h] = rect;
        let hx = x + fraction.clamp(0.0, 1.0) * (w - 8.0);
        self.sprite(if hover { HANDLE_HOVER } else { HANDLE }, [hx, y, 8.0, h]);
        self.centred(label, rect, INK);
        self.widgets.push(Widget { rect, action });
    }
    /// The game's text field showing `value` (stars for a password), focused or not.
    fn field(&mut self, rect: [f32; 4], value: &str, hint: &str, secret: bool, focus: bool, action: Action) {
        self.sprite(if focus { FIELD_FOCUS } else { FIELD }, rect);
        let shown = if secret { "*".repeat(value.chars().count()) } else { value.to_owned() };
        let (mut text, rgb) = if shown.is_empty() && !focus { (hint.to_owned(), DARK) } else { (shown, INK) };
        while !text.is_empty() && self.width(&text) > rect[2] - 12.0 {
            text.remove(0);
        }
        let caret = if focus { "_" } else { "" };
        self.text(&format!("{text}{caret}"), [rect[0] + 4.0, rect[1] + (rect[3] - 8.0) / 2.0], rgb);
        self.widgets.push(Widget { rect, action });
    }
    /// A small microphone at `(x, y)`, 8 x 12, in `argb`.
    fn microphone(&mut self, x: f32, y: f32, argb: u32) {
        self.fill([x + 2.0, y, 4.0, 7.0], argb);
        self.fill([x, y + 4.0, 1.0, 3.0], argb);
        self.fill([x + 7.0, y + 4.0, 1.0, 3.0], argb);
        self.fill([x + 1.0, y + 7.0, 6.0, 1.0], argb);
        self.fill([x + 3.5, y + 8.0, 1.0, 2.0], argb);
        self.fill([x + 1.5, y + 10.0, 5.0, 1.0], argb);
    }
}

pub const KINDS: [&str; 3] = ["Normal", "Open", "Isolated"];

/// Where a slider's handle sits for each setting, 0 to 1, and back.
pub fn slider_fraction(slider: Slider, s: &Settings) -> f32 {
    match slider {
        Slider::Volume => s.volume as f32 / 200.0,
        Slider::Gain => (s.mic_gain as f32 - 25.0) / 475.0,
        Slider::Threshold => (s.threshold as f32 + 80.0) / 80.0,
    }
}

/// The setting a slider tapped at `fraction` (0 to 1) sets, in its steps: volume 0-200% by 5,
/// gain 25-500% by 25, the activation level -80 to 0 dB by 1.
pub fn slider_value(slider: Slider, fraction: f32) -> i32 {
    let f = fraction.clamp(0.0, 1.0);
    match slider {
        Slider::Volume => ((f * 200.0 / 5.0).round() * 5.0) as i32,
        Slider::Gain => 25 + ((f * 475.0 / 25.0).round() * 25.0) as i32,
        Slider::Threshold => (-80.0 + f * 80.0).round() as i32,
    }
}

impl Menu {
    pub fn open(&mut self, opened_by: Option<char>) {
        self.open = true;
        self.skip_char = opened_by;
        self.message = None;
    }

    pub fn reset(&mut self) {
        self.open = false;
        self.focus = None;
        self.joining = None;
        self.join_password.clear();
        self.skip_char = None;
        self.widgets.clear();
    }

    pub fn close(&mut self) {
        self.reset();
        self.close_requested = true;
    }

    pub fn take_close(&mut self) -> bool {
        std::mem::take(&mut self.close_requested)
    }

    /// The game's screen is (`ours`) or isn't the menu: forget a menu the
    /// game closed.
    pub fn sync_open(&mut self, ours: bool) {
        if !ours && self.open {
            self.reset();
        }
    }

    /// The action under the pointer, from the last frame drawn.
    pub fn hit(&self, at: [f32; 2]) -> Option<Action> {
        self.widgets.iter().rev().find(|w| inside(w.rect, at)).map(|w| w.action.clone())
    }

    /// Where the widget doing `action` was drawn last.
    pub fn hit_rect(&self, action: &Action) -> Option<[f32; 4]> {
        self.widgets.iter().find(|w| &w.action == action).map(|w| w.rect)
    }

    /// How far along the slider doing `action` the point `at` is, 0 to 1 (the handle's middle).
    pub fn fraction(&self, action: &Action, at: [f32; 2]) -> Option<f32> {
        let [x, _, w, _] = self.hit_rect(action)?;
        Some(((at[0] - x - 4.0) / (w - 8.0)).clamp(0.0, 1.0))
    }

    fn field_mut(&mut self, f: Field) -> &mut String {
        match f {
            Field::Name => &mut self.name,
            Field::Password => &mut self.password,
            Field::JoinPassword => &mut self.join_password,
        }
    }

    pub fn char_typed(&mut self, c: char) {
        if let Some(skip) = self.skip_char.take()
            && skip.eq_ignore_ascii_case(&c)
        {
            return;
        }
        let Some(f) = self.focus else { return };
        let max = if f == Field::Name { NAME_MAX } else { PASSWORD_MAX };
        let field = self.field_mut(f);
        if !c.is_control() && field.chars().count() < max && !(f != Field::Name && c == ' ') {
            field.push(c);
        }
    }

    /// Editing keys. Returns the action Enter asks for, if any.
    pub fn key(&mut self, key: &str, shift: bool) -> Option<Action> {
        self.skip_char = None;
        match key {
            "key.keyboard.escape" => {
                if self.joining.is_some() {
                    return Some(Action::CancelJoin);
                }
                if self.focus.is_some() {
                    self.focus = None;
                    return None;
                }
                return Some(Action::Close);
            }
            "key.keyboard.backspace" => {
                if let Some(f) = self.focus {
                    self.field_mut(f).pop();
                }
            }
            "key.keyboard.tab" => {
                // The fields are the Groups tab's.
                self.tab = Tab::Groups;
                self.focus = match (self.focus, shift) {
                    (Some(Field::JoinPassword), _) => Some(Field::JoinPassword),
                    (Some(Field::Name), false) | (Some(Field::Password), true) => Some(Field::Password),
                    _ => Some(Field::Name),
                };
                if self.joining.is_some() {
                    self.focus = Some(Field::JoinPassword);
                }
            }
            "key.keyboard.enter" | "key.keyboard.keypad.enter" => match self.focus {
                Some(Field::JoinPassword) => return Some(Action::JoinWithPassword),
                Some(_) => return Some(Action::Create),
                None => {}
            },
            "key.keyboard.up" => self.scroll = self.scroll.saturating_sub(1),
            "key.keyboard.down" => self.scroll += 1,
            _ => {}
        }
        None
    }

    /// Whether typing goes into a field, so other keys (the menu key) type.
    pub fn typing(&self) -> bool {
        self.focus.is_some()
    }

    pub fn scroll_by(&mut self, steps: i32) {
        self.scroll = (self.scroll as i32 - steps).max(0) as usize;
    }

    pub fn draw(&mut self, view: &View, size: [f32; 2], mouse: [f32; 2], measure: &dyn Fn(&str) -> f32) -> Vec<Draw> {
        let mut out = Out { draws: Vec::new(), widgets: Vec::new(), mouse, measure };
        let w = (size[0] - 12.0).min(420.0);
        let h = (size[1] - 10.0).min(244.0);
        let (x0, y0) = (((size[0] - w) / 2.0).floor(), ((size[1] - h) / 2.0).floor());
        out.fill([0.0, 0.0, size[0], size[1]], SHADE);
        out.fill([x0 - 1.0, y0 - 1.0, w + 2.0, h + 2.0], PANEL_EDGE);
        out.fill([x0, y0, w, h], PANEL);
        out.fill([x0, y0, w, 22.0], HEADER);
        out.fill([x0, y0 + 22.0, w, 1.0], PANEL_EDGE);

        // The header: the microphone, the name, the connection with its light, and close.
        out.microphone(x0 + 8.0, y0 + 5.0, ACCENT);
        out.text(TITLE, [x0 + 21.0, y0 + 7.0], GOLD);
        let (status, rgb) = &view.status;
        let sx = x0 + 21.0 + out.width(TITLE) + 12.0;
        out.fill([sx, y0 + 9.0, 4.0, 4.0], 0xFF00_0000 | *rgb);
        let st = out.fit(status, x0 + w - 26.0 - (sx + 8.0));
        out.text(&st, [sx + 8.0, y0 + 7.0], *rgb);
        out.button([x0 + w - 20.0, y0 + 2.0, 18.0, 18.0], "x", Action::Close, None);

        // The tabs.
        let tw = ((w - 16.0) / 3.0).floor();
        for (i, (label, tab)) in [("Voice", Tab::Voice), ("Groups", Tab::Groups), ("Keys", Tab::Keys)]
            .into_iter()
            .enumerate()
        {
            let rect = [x0 + 6.0 + i as f32 * (tw + 2.0), y0 + 26.0, tw, 18.0];
            out.tab(rect, label, Action::Tab(tab), self.tab == tab);
        }

        let (x, top, cw, bottom) = (x0 + 8.0, y0 + 50.0, w - 16.0, y0 + h - 6.0);
        match self.tab {
            Tab::Voice => self.draw_voice(&mut out, view, x, top, cw, bottom),
            Tab::Groups => self.draw_groups(&mut out, view, x, top, cw, bottom),
            Tab::Keys => self.draw_keys(&mut out, view, x, top, cw, bottom),
        }
        self.widgets = out.widgets;
        out.draws
    }

    /// The Voice tab: the switches on the left, the sliders and the microphone's level on the
    /// right.
    fn draw_voice(&self, out: &mut Out<'_>, view: &View, x: f32, top: f32, w: f32, bottom: f32) {
        let s = &view.settings;
        let col = ((w - 6.0) / 2.0).floor();
        let step = ((bottom - top - 14.0) / 4.0).clamp(H + 2.0, H + 6.0);
        let left = [
            (format!("Microphone: {}", if s.muted { "Muted" } else { "On" }), Action::Mute, Some(!s.muted)),
            (format!("Voice chat: {}", if s.disabled { "Off" } else { "On" }), Action::Disable, Some(!s.disabled)),
            (
                format!("Talk: {}", if s.mode == Mode::Voice { "When I speak" } else { "Hold the key" }),
                Action::Mode,
                None,
            ),
            (format!("Group names: {}", if s.group_labels { "On" } else { "Off" }), Action::Labels, Some(s.group_labels)),
        ];
        for (i, (label, action, state)) in left.into_iter().enumerate() {
            out.button([x, top + i as f32 * step, col, H], &label, action, state);
        }

        let rx = x + col + 6.0;
        let sliders = [
            (Slider::Volume, format!("Voice volume: {}%", s.volume)),
            (Slider::Gain, format!("Microphone gain: {}%", s.mic_gain)),
            (Slider::Threshold, format!("Speak above: {} dB", s.threshold)),
        ];
        for (i, (slider, label)) in sliders.into_iter().enumerate() {
            out.slider([rx, top + i as f32 * step, col, H], &label, slider_fraction(slider, s), Action::Slide(slider));
        }
        // The microphone's level against the activation level, -80 to 0 dB.
        let my = top + 3.0 * step;
        out.text("Microphone level", [rx, my], GREY);
        let bar = [rx, my + 11.0, col, 7.0];
        out.fill(bar, WELL);
        let at = |db: f32| ((db + 80.0) / 80.0).clamp(0.0, 1.0) * (col - 2.0);
        let voice = s.mode == Mode::Voice;
        let loud = view.level_db > s.threshold as f32;
        let level = if loud || !voice { ON } else { 0xFF4A_7A4A };
        out.fill([rx + 1.0, my + 12.0, at(view.level_db), 5.0], level);
        if voice {
            out.fill([rx + 1.0 + at(s.threshold as f32), my + 10.0, 1.0, 9.0], ACCENT);
        }
        let hint = if s.muted {
            "Muted: nobody hears you"
        } else if voice {
            "Talks when the bar passes the yellow line"
        } else {
            "Hold the push-to-talk key to talk"
        };
        out.text(&out.fit(hint, w), [x, bottom - 9.0], DARK);
    }

    /// The Groups tab: the server's groups to join, the one you're in to leave, and a form for
    /// a new one.
    fn draw_groups(&self, out: &mut Out<'_>, view: &View, x: f32, top: f32, w: f32, bottom: f32) {
        let mut y = top;
        if !view.connected || !view.groups_enabled {
            let why = if view.connected { "Groups are off on this server" } else { "Ferrite VC isn't connected" };
            out.centred(why, [x, y, w, 20.0], GREY);
            return;
        }
        if let Some(ours) = view.groups.iter().find(|g| g.ours) {
            let label = out.fit(&format!("You're in {}", ours.name), w - 70.0);
            out.text(&label, [x, y + 6.0], GREEN);
            out.button([x + w - 64.0, y, 64.0, H], "Leave", Action::Leave, None);
            y += H + 4.0;
        }
        // The form for a new group at the bottom, the last message above it.
        let form_top = bottom - (11.0 + H + 3.0 + H);
        let list_bottom = form_top - if self.message.is_some() { 14.0 } else { 4.0 };
        let fits = (((list_bottom - y) / ROW).floor().max(1.0)) as usize;
        let scroll = self.scroll.min(view.groups.len().saturating_sub(fits));
        if view.groups.is_empty() {
            out.centred("No groups yet: make one below", [x, y, w, 20.0], GREY);
        }
        let arrows = view.groups.len() > fits;
        let rw = if arrows { w - 24.0 } else { w };
        for g in view.groups.iter().skip(scroll).take(fits) {
            out.fill([x, y, rw, ROW - 2.0], if g.ours { ROW_OURS } else { ROW_FILL });
            if self.joining == Some(g.id) {
                let fw = rw - 112.0;
                out.field([x + 2.0, y + 2.0, fw, H], &self.join_password, &format!("password for {}", g.name), true, self.focus == Some(Field::JoinPassword), Action::Focus(Field::JoinPassword));
                out.button([x + fw + 6.0, y + 2.0, 50.0, H], "Join", Action::JoinWithPassword, None);
                out.button([x + fw + 58.0, y + 2.0, 50.0, H], "Cancel", Action::CancelJoin, None);
            } else {
                let lock = if g.has_password { " (password)" } else { "" };
                out.text(&out.fit(&format!("{}{lock}", g.name), rw - 62.0), [x + 4.0, y + 3.0], if g.ours { GREEN } else { INK });
                let members = if g.members.is_empty() { "nobody yet".to_owned() } else { g.members.join(", ") };
                out.text(&out.fit(&members, rw - 62.0), [x + 4.0, y + 13.0], GREY);
                if !g.ours {
                    out.button([x + rw - 54.0, y + 2.0, 52.0, H], "Join", Action::Join(g.id), None);
                }
            }
            y += ROW;
        }
        if arrows {
            out.button([x + w - 20.0, top, 20.0, H], "^", Action::Scroll(-1), None);
            out.button([x + w - 20.0, list_bottom - H, 20.0, H], "v", Action::Scroll(1), None);
            let more = format!("{}-{}", scroll + 1, (scroll + fits).min(view.groups.len()));
            let mw = out.width(&more);
            out.text(&more, [x + w - 10.0 - mw / 2.0, (top + list_bottom) / 2.0 - 4.0], DARK);
        }
        if let Some((m, rgb)) = &self.message {
            out.text(&out.fit(m, w), [x, form_top - 11.0], *rgb);
        }
        out.text("New group", [x, form_top], GOLD);
        let fy = form_top + 11.0;
        let third = ((w - 8.0) / 3.0).floor();
        out.field([x, fy, third * 2.0 + 4.0, H], &self.name, "name", false, self.focus == Some(Field::Name), Action::Focus(Field::Name));
        out.button([x + third * 2.0 + 8.0, fy, third, H], &format!("Type: {}", KINDS[self.kind as usize % 3]), Action::Kind, None);
        let by = fy + H + 3.0;
        out.field([x, by, third * 2.0 + 4.0, H], &self.password, "password (optional)", true, self.focus == Some(Field::Password), Action::Focus(Field::Password));
        out.button([x + third * 2.0 + 8.0, by, third, H], "Create", Action::Create, None);
    }

    /// The Keys tab: what each hotkey does and the key it's on.
    fn draw_keys(&self, out: &mut Out<'_>, view: &View, x: f32, top: f32, w: f32, bottom: f32) {
        let mut y = top;
        for (what, key) in &view.hotkeys {
            if y + 18.0 > bottom - 12.0 {
                break;
            }
            out.fill([x, y, w, 18.0], ROW_FILL);
            out.text(&out.fit(what, w * 0.6), [x + 6.0, y + 5.0], INK);
            let unbound = key == "not bound";
            let kw = out.width(key).max(20.0) + 10.0;
            let cap = [x + w - kw - 4.0, y + 2.0, kw, 14.0];
            out.fill(cap, if unbound { 0xFF20_2028 } else { 0xFF3A_3A46 });
            out.centred(key, cap, if unbound { DARK } else { GOLD });
            y += 20.0;
        }
        out.text(&out.fit("Change the keys in Options > Mods > Ferrite VC", w), [x, bottom - 9.0], DARK);
    }

    /// What a click on the menu's own parts does; the rest is the voice
    /// chat's to do (returned).
    pub fn act(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::Focus(f) => self.focus = Some(f),
            Action::Kind => self.kind = (self.kind + 1) % 3,
            Action::Scroll(d) => self.scroll_by(-d),
            Action::Tab(tab) => {
                self.tab = tab;
                self.focus = None;
            }
            Action::CancelJoin => {
                self.joining = None;
                self.join_password.clear();
                self.focus = None;
            }
            Action::Close => self.close(),
            other => {
                if !matches!(other, Action::JoinWithPassword | Action::Create) {
                    self.focus = None;
                }
                return Some(other);
            }
        }
        None
    }

    pub fn say(&mut self, text: impl Into<String>, error: bool) {
        self.message = Some((text.into(), if error { RED } else { GREEN }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> View {
        View {
            status: ("Connected".into(), 0x55FF55),
            settings: Settings::default(),
            level_db: -30.0,
            connected: true,
            groups_enabled: true,
            groups: vec![
                GroupRow { id: 1, name: "Builders".into(), has_password: false, members: vec!["Alex".into()], ours: false },
                GroupRow { id: 2, name: "Secret".into(), has_password: true, members: vec![], ours: false },
            ],
            hotkeys: vec![("Push to talk".into(), "Caps Lock".into()), ("Whisper".into(), "not bound".into())],
        }
    }

    fn texts(draws: &[Draw]) -> Vec<String> {
        draws.iter().filter_map(|d| if let Draw::Text(t, ..) = d { Some(t.clone()) } else { None }).collect()
    }

    fn centre(m: &Menu, action: &Action) -> [f32; 2] {
        let w = m.widgets.iter().find(|w| &w.action == action).unwrap();
        [w.rect[0] + w.rect[2] / 2.0, w.rect[1] + w.rect[3] / 2.0]
    }

    const MEASURE: fn(&str) -> f32 = |t| t.len() as f32 * 6.0;

    /// Every tab at a phone's GUI size and a desktop's: its texts, and every control on screen.
    #[test]
    fn each_tab_shows_its_part_and_fits_the_screen() {
        for size in [[480.0, 270.0], [427.0, 196.0]] {
            let mut m = Menu::default();
            m.open(Some('v'));
            let t = texts(&m.draw(&view(), size, [0.0, 0.0], &MEASURE));
            for want in [TITLE, "Connected", "Voice", "Groups", "Keys", "Microphone: On", "Microphone level"] {
                assert!(t.iter().any(|x| x == want), "{want} missing from {t:?}");
            }
            let on_screen = |m: &Menu| {
                m.widgets.iter().all(|w| {
                    w.rect[0] >= 0.0 && w.rect[1] >= 0.0 && w.rect[0] + w.rect[2] <= size[0] && w.rect[1] + w.rect[3] <= size[1]
                })
            };
            assert!(on_screen(&m), "{size:?}");
            assert!(m.hit_rect(&Action::Slide(Slider::Volume)).is_some());

            m.act(Action::Tab(Tab::Groups));
            let t = texts(&m.draw(&view(), size, [0.0, 0.0], &MEASURE));
            for want in ["Builders", "Alex", "Secret (password)", "New group"] {
                assert!(t.iter().any(|x| x == want), "{want} missing from {t:?}");
            }
            assert!(on_screen(&m), "{size:?}");
            let join = centre(&m, &Action::Join(1));
            assert_eq!(m.hit(join), Some(Action::Join(1)));

            m.act(Action::Tab(Tab::Keys));
            let t = texts(&m.draw(&view(), size, [0.0, 0.0], &MEASURE));
            for want in ["Push to talk", "Caps Lock", "not bound"] {
                assert!(t.iter().any(|x| x == want), "{want} missing from {t:?}");
            }
        }
    }

    #[test]
    fn a_slider_sets_where_it_is_tapped() {
        let mut m = Menu::default();
        m.open(None);
        m.draw(&view(), [480.0, 270.0], [0.0, 0.0], &MEASURE);
        let action = Action::Slide(Slider::Volume);
        let [x, y, w, _] = m.hit_rect(&action).unwrap();
        assert_eq!(m.hit([x + w / 2.0, y + 5.0]), Some(action.clone()));
        let half = m.fraction(&action, [x + w / 2.0, y + 5.0]).unwrap();
        assert!((half - 0.5).abs() < 0.01, "{half}");
        assert_eq!(slider_value(Slider::Volume, half), 100);
        assert_eq!(slider_value(Slider::Volume, 1.0), 200);
        assert_eq!(slider_value(Slider::Gain, 0.0), 25);
        assert_eq!(slider_value(Slider::Gain, 1.0), 500);
        assert_eq!(slider_value(Slider::Threshold, 0.375), -50);
        // The handle shows each setting where a tap there would set it.
        let s = Settings::default();
        for slider in [Slider::Volume, Slider::Gain, Slider::Threshold] {
            let f = slider_fraction(slider, &s);
            let value = slider_value(slider, f);
            let now = match slider {
                Slider::Volume => s.volume as i32,
                Slider::Gain => s.mic_gain as i32,
                Slider::Threshold => s.threshold,
            };
            assert_eq!(value, now, "{slider:?}");
        }
    }

    #[test]
    fn typing_a_new_group_and_creating_it() {
        let mut m = Menu::default();
        m.open(Some('v'));
        m.char_typed('v');
        assert_eq!(m.act(Action::Focus(Field::Name)), None);
        for c in "Team A".chars() {
            m.char_typed(c);
        }
        m.key("key.keyboard.tab", false);
        assert_eq!(m.tab, Tab::Groups);
        for c in "p w".chars() {
            m.char_typed(c);
        }
        assert_eq!((m.name.as_str(), m.password.as_str()), ("Team A", "pw"));
        m.act(Action::Kind);
        assert_eq!(m.kind, 1);
        assert_eq!(m.key("key.keyboard.enter", false), Some(Action::Create));
        assert_eq!(m.key("key.keyboard.escape", false), None);
        assert_eq!(m.key("key.keyboard.escape", false), Some(Action::Close));
    }
}
