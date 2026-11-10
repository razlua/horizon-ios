//! iOS: the screen shown before the game starts (game/ios_boot.rs): the first start's unpacking
//! of the resources and the Microsoft sign-in. Drawn like `no_resources`, with the bundled fonts,
//! so it shows before any of Minecraft's files are there.
use crate::ui_theme::{Face, Ui, color};

/// What a tap on the screen does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Open Microsoft's page with the code filled in.
    OpenLink,
    /// Ask Microsoft for a new code.
    Retry,
    /// Start without an account.
    Offline,
}

/// What the screen says.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Screen {
    pub title: String,
    pub lines: Vec<String>,
    /// Shown large: the code to enter.
    pub code: Option<String>,
    /// Shown below the text, muted: what is going on in the background.
    pub status: Option<String>,
    pub buttons: Vec<(&'static str, Action)>,
}

const COLUMN: f32 = 620.0;
const TITLE_SIZE: f32 = horizon_theme::TITLE.px;
const TITLE_LINE: f32 = 40.0;
const BODY: f32 = horizon_theme::BODY.px;
const LINE: f32 = 24.0;
const CODE_SIZE: f32 = 44.0;
const CODE_LINE: f32 = 60.0;
const BUTTON_TEXT: f32 = horizon_theme::UI.px;
const CONTROL: f32 = horizon_theme::CONTROL_LARGE;
const GAP: f32 = 12.0;

struct Placed {
    x: f32,
    y: f32,
    width: f32,
    lines: Vec<String>,
    buttons: Vec<(f32, f32, f32, &'static str, Action)>,
}

impl Screen {
    fn place(&self, ui: &Ui) -> Placed {
        let width = COLUMN.min(ui.w - 64.0).max(160.0);
        let x = ((ui.w - width) / 2.0).round();
        let lines: Vec<String> = self
            .lines
            .iter()
            .flat_map(|l| wrap(ui, Face::Regular, BODY, l, width))
            .collect();
        let code_h = if self.code.is_some() { CODE_LINE + 8.0 } else { 0.0 };
        let buttons_h = if self.buttons.is_empty() { 0.0 } else { 24.0 + CONTROL };
        let status_h = if self.status.is_some() { 16.0 + LINE } else { 0.0 };
        let height = TITLE_LINE + 16.0 + lines.len() as f32 * LINE + code_h + buttons_h + status_h;
        let y = ((ui.h - height) / 2.0).max(16.0).round();
        let mut by = y + TITLE_LINE + 16.0 + lines.len() as f32 * LINE + code_h + 24.0;
        if self.buttons.is_empty() {
            by -= 24.0;
        }
        let mut bx = x;
        let buttons = self
            .buttons
            .iter()
            .map(|&(label, action)| {
                let w = (ui.measure(Face::Medium, BUTTON_TEXT, label) + 48.0).round();
                let placed = (bx, by, w, label, action);
                bx += w + GAP;
                placed
            })
            .collect();
        Placed {
            x,
            y,
            width,
            lines,
            buttons,
        }
    }

    pub fn draw(&self, ui: &mut Ui) {
        let placed = self.place(ui);
        ui.rounded(0.0, 0.0, ui.w, ui.h, 0.0, color::BG);
        let mut y = placed.y;
        ui.text_line(
            Face::Medium,
            TITLE_SIZE,
            TITLE_LINE,
            placed.x,
            y,
            color::TEXT,
            &self.title,
        );
        y += TITLE_LINE + 16.0;
        for line in &placed.lines {
            ui.text_line(Face::Regular, BODY, LINE, placed.x, y, color::TEXT, line);
            y += LINE;
        }
        if let Some(code) = &self.code {
            y += 8.0;
            ui.text_line(Face::Medium, CODE_SIZE, CODE_LINE, placed.x, y, color::ACCENT, code);
        }
        for &(bx, by, w, label, action) in &placed.buttons {
            let hovered = inside(ui.cursor, bx, by, w);
            let (plate, ink) = match (action, hovered) {
                (Action::OpenLink, false) => (color::ACCENT, color::ON_ACCENT),
                (Action::OpenLink, true) => {
                    (color::with_alpha(color::ACCENT, 0.85), color::ON_ACCENT)
                }
                (_, false) => (color::RAISED, color::TEXT),
                (_, true) => (color::HOVER, color::TEXT),
            };
            ui.rounded(bx, by, w, CONTROL, CONTROL / 2.0, plate);
            let lw = ui.measure(Face::Medium, BUTTON_TEXT, label);
            let lx = bx + ((w - lw) / 2.0).round();
            ui.text_line(Face::Medium, BUTTON_TEXT, CONTROL, lx, by, ink, label);
        }
        if let Some(status) = &self.status {
            let sy = placed
                .buttons
                .first()
                .map_or(y + 16.0, |b| b.1 + CONTROL + 16.0);
            for (i, line) in wrap(ui, Face::Regular, BODY, status, placed.width)
                .iter()
                .enumerate()
            {
                let line_y = sy + i as f32 * LINE;
                ui.text_line(Face::Regular, BODY, LINE, placed.x, line_y, color::MUTED, line);
            }
        }
    }

    /// The button under the cursor (`ui.cursor`, in dp), if any.
    pub fn click(&self, ui: &Ui) -> Option<Action> {
        self.place(ui)
            .buttons
            .iter()
            .find(|b| inside(ui.cursor, b.0, b.1, b.2))
            .map(|b| b.4)
    }
}

fn inside((cx, cy): (f32, f32), x: f32, y: f32, w: f32) -> bool {
    cx >= x && cx < x + w && cy >= y && cy < y + CONTROL
}

/// `text` broken at spaces into lines no wider than `width` dp.
fn wrap(ui: &Ui, face: Face, size: f32, text: &str, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        let candidate = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && ui.measure(face, size, &candidate) > width {
            lines.push(std::mem::take(&mut line));
            line = word.to_owned();
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
