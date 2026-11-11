//! iOS: the screen shown before the game starts (game/ios_boot.rs): the first start's unpacking
//! of the resources, the account picker and the Microsoft sign-in. Drawn like `no_resources`,
//! with the bundled fonts, so it shows before any of Minecraft's files are there.
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
    /// Play as the saved account at this index.
    Play(usize),
    /// Sign another Microsoft account in.
    AddAccount,
    /// The account buttons remove instead of playing (and back).
    RemoveMode,
    /// Forget the saved account at this index.
    Remove(usize),
    /// Back to the account list.
    Back,
    /// Start the game files' download again (it goes on where it stopped).
    RetrySetup,
}

/// How a button looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Look {
    /// The accent colour: what the screen is for.
    Primary,
    #[default]
    Plain,
    /// Removing something.
    Danger,
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
    /// The unpacking's progress, 0 to 1, as a bar under the status.
    pub progress: Option<f32>,
    pub buttons: Vec<(String, Action, Look)>,
}

const COLUMN: f32 = 640.0;
const TITLE_SIZE: f32 = horizon_theme::TITLE.px;
const TITLE_LINE: f32 = 40.0;
const BODY: f32 = horizon_theme::BODY.px;
const LINE: f32 = 24.0;
const CODE_SIZE: f32 = 44.0;
const CODE_LINE: f32 = 60.0;
const BUTTON_TEXT: f32 = horizon_theme::UI.px;
const CONTROL: f32 = horizon_theme::CONTROL_LARGE;
const GAP: f32 = 12.0;
const BAR: f32 = 8.0;

struct Placed {
    x: f32,
    y: f32,
    width: f32,
    lines: Vec<String>,
    buttons: Vec<(f32, f32, f32, usize)>,
    /// Where the status line goes.
    status_y: f32,
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
        // The buttons in rows that fit the column.
        let mut rows: Vec<Vec<(f32, usize)>> = Vec::new();
        let mut row_w = 0.0;
        for (i, (label, _, _)) in self.buttons.iter().enumerate() {
            let w = (ui.measure(Face::Medium, BUTTON_TEXT, label) + 48.0).round();
            if rows.is_empty() || row_w + GAP + w > width {
                rows.push(Vec::new());
                row_w = w;
            } else {
                row_w += GAP + w;
            }
            if let Some(row) = rows.last_mut() {
                row.push((w, i));
            }
        }
        let code_h = if self.code.is_some() { CODE_LINE + 8.0 } else { 0.0 };
        let buttons_h = if rows.is_empty() {
            0.0
        } else {
            24.0 + rows.len() as f32 * CONTROL + (rows.len() - 1) as f32 * GAP
        };
        let status_h = if self.status.is_some() { 16.0 + LINE } else { 0.0 }
            + if self.progress.is_some() { 12.0 + BAR } else { 0.0 };
        let height = TITLE_LINE + 16.0 + lines.len() as f32 * LINE + code_h + buttons_h + status_h;
        let y = ((ui.h - height) / 2.0).max(16.0).round();
        let mut by = y + TITLE_LINE + 16.0 + lines.len() as f32 * LINE + code_h + 24.0;
        let mut buttons = Vec::new();
        for row in &rows {
            let mut bx = x;
            for &(w, i) in row {
                buttons.push((bx, by, w, i));
                bx += w + GAP;
            }
            by += CONTROL + GAP;
        }
        let status_y = if rows.is_empty() {
            y + TITLE_LINE + 16.0 + lines.len() as f32 * LINE + code_h + 16.0
        } else {
            by - GAP + 16.0
        };
        Placed {
            x,
            y,
            width,
            lines,
            buttons,
            status_y,
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
        for &(bx, by, w, i) in &placed.buttons {
            let (label, _, look) = &self.buttons[i];
            let hovered = inside(ui.cursor, bx, by, w);
            let (plate, ink) = match (look, hovered) {
                (Look::Primary, false) => (color::ACCENT, color::ON_ACCENT),
                (Look::Primary, true) => {
                    (color::with_alpha(color::ACCENT, 0.85), color::ON_ACCENT)
                }
                (Look::Danger, _) => (color::hex(0xC83C3C, 235), color::TEXT),
                (Look::Plain, false) => (color::RAISED, color::TEXT),
                (Look::Plain, true) => (color::HOVER, color::TEXT),
            };
            ui.rounded(bx, by, w, CONTROL, CONTROL / 2.0, plate);
            let lw = ui.measure(Face::Medium, BUTTON_TEXT, label);
            let lx = bx + ((w - lw) / 2.0).round();
            ui.text_line(Face::Medium, BUTTON_TEXT, CONTROL, lx, by, ink, label);
        }
        let mut sy = placed.status_y;
        if let Some(status) = &self.status {
            for line in wrap(ui, Face::Regular, BODY, status, placed.width) {
                ui.text_line(Face::Regular, BODY, LINE, placed.x, sy, color::MUTED, &line);
                sy += LINE;
            }
        }
        if let Some(progress) = self.progress {
            sy += 12.0;
            ui.rounded(placed.x, sy, placed.width, BAR, BAR / 2.0, color::RAISED);
            let filled = (placed.width * progress.clamp(0.0, 1.0)).max(BAR);
            ui.rounded(placed.x, sy, filled, BAR, BAR / 2.0, color::ACCENT);
        }
    }

    /// The button under the cursor (`ui.cursor`, in dp), if any.
    pub fn click(&self, ui: &Ui) -> Option<Action> {
        self.place(ui)
            .buttons
            .iter()
            .find(|b| inside(ui.cursor, b.0, b.1, b.2))
            .map(|b| self.buttons[b.3].1)
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
