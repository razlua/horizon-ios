//! Edit HUD (the iOS build's title, in Quit Game's place): the player moves, adds, removes and
//! resizes the touch buttons (touch.rs), moves and resizes the move stick's area, moves the Esc
//! button, sets how opaque the buttons are, and keeps up to three layouts. They are kept in
//! `touch-controls.json` in the game folder.
//!
//! The logic is plain data in and out, as in touch.rs: `Editor::touch` takes a finger in window
//! pixels and says when the player is done.

use super::touch::{ACTIONS, HudLayout, LAYOUTS, Placed};
use winit::event::TouchPhase;

/// The editor's own buttons, along the top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tool {
    Add,
    Remove,
    Smaller,
    Bigger,
    Fainter,
    Bolder,
    /// The next of the saved layouts.
    Layout,
    Reset,
    Done,
}

pub(super) const TOOLS: [(Tool, &str); 9] = [
    (Tool::Add, "Add"),
    (Tool::Remove, "Remove"),
    (Tool::Smaller, "Smaller"),
    (Tool::Bigger, "Bigger"),
    (Tool::Fainter, "Fainter"),
    (Tool::Bolder, "Bolder"),
    (Tool::Layout, "Layout"),
    (Tool::Reset, "Reset"),
    (Tool::Done, "Done"),
];

/// What is selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Sel {
    Button(usize),
    Stick,
    Esc,
}

/// A button's size for one just added, a share of the screen's short side.
const NEW_SIZE: f32 = 0.11;
const MIN_SIZE: f32 = 0.06;
const MAX_SIZE: f32 = 0.30;
const MIN_OPACITY: f32 = 0.2;

fn inside([x, y, w, h]: [f32; 4], [px, py]: [f32; 2]) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}

/// The unit the editor's own parts are measured in: a share of the short side.
fn unit([w, h]: [f32; 2]) -> f32 {
    w.min(h) * 0.15
}

/// The tools' rectangles near the top, centred, in as many rows as the width needs: under the
/// default top row of buttons and the Esc button, above the move stick's area.
pub(super) fn toolbar(size: [f32; 2]) -> Vec<(Tool, &'static str, [f32; 4])> {
    let u = unit(size);
    let (bw, bh, gap) = (u * 1.15, u * 0.5, u * 0.12);
    let per_row = (((size[0] * 0.94 + gap) / (bw + gap)).floor() as usize).clamp(1, TOOLS.len());
    TOOLS
        .chunks(per_row)
        .enumerate()
        .flat_map(|(row, tools)| {
            let total = tools.len() as f32 * bw + (tools.len() - 1) as f32 * gap;
            let left = (size[0] - total) / 2.0;
            let y = u * 1.0 + row as f32 * (bh + gap);
            tools.iter().enumerate().map(move |(i, &(tool, label))| {
                (tool, label, [left + i as f32 * (bw + gap), y, bw, bh])
            })
        })
        .collect()
}

/// Where the toolbar ends, for the line of help under it.
pub(super) fn toolbar_bottom(size: [f32; 2]) -> f32 {
    toolbar(size)
        .iter()
        .map(|(_, _, r)| r[1] + r[3])
        .fold(0.0, f32::max)
}

/// The Add panel and its tiles: every action not on the screen yet.
pub(super) fn picker(size: [f32; 2], placed: &[Placed]) -> ([f32; 4], Vec<(usize, [f32; 4])>) {
    let [w, h] = size;
    let u = unit(size);
    let panel = [w * 0.12, h * 0.17, w * 0.76, h * 0.78];
    let (tw, gap) = (u * 1.85, u * 0.1);
    let columns = (((panel[2] - gap) / (tw + gap)).floor() as usize).max(1);
    let left = panel[0] + (panel[2] - (columns as f32 * (tw + gap) - gap)) / 2.0;
    let missing: Vec<usize> = ACTIONS
        .iter()
        .enumerate()
        .filter(|(_, a)| !placed.iter().any(|p| p.id == a.id))
        .map(|(i, _)| i)
        .collect();
    // Every tile fits the panel: on a squarer screen (an iPad) the rows grow thinner.
    let top = panel[1] + gap * 3.0 + u * 0.3;
    let rows = missing.len().div_ceil(columns).max(1) as f32;
    let room = panel[1] + panel[3] - gap - top;
    let th = (u * 0.45).min((room - (rows - 1.0) * gap) / rows);
    let tiles = missing
        .into_iter()
        .enumerate()
        .map(|(n, i)| {
            let (col, row) = (n % columns, n / columns);
            let rect = [
                left + col as f32 * (tw + gap),
                top + row as f32 * (th + gap),
                tw,
                th,
            ];
            (i, rect)
        })
        .collect();
    (panel, tiles)
}

/// The layouts being edited.
pub(super) struct Editor {
    /// Every saved layout (the defaults where none was saved).
    pub layouts: Vec<HudLayout>,
    pub active: usize,
    pub selected: Option<Sel>,
    /// The Add panel is open.
    pub picking: bool,
    /// The finger moving the selection, and where it holds it (from the button's centre, or the
    /// stick area's corner).
    drag: Option<(u64, [f32; 2])>,
}

impl Editor {
    pub(super) fn new(layouts: Vec<HudLayout>, active: usize) -> Self {
        let active = active.min(layouts.len().saturating_sub(1));
        Self {
            layouts,
            active,
            selected: None,
            picking: false,
            drag: None,
        }
    }

    /// The layout on the screen.
    pub(super) fn current(&self) -> &HudLayout {
        &self.layouts[self.active]
    }

    fn current_mut(&mut self) -> &mut HudLayout {
        &mut self.layouts[self.active]
    }

    /// One finger. `defaults` is the layout Reset puts back. When the player taps Done: the
    /// layouts and the one to use.
    pub(super) fn touch(
        &mut self,
        id: u64,
        phase: TouchPhase,
        at: [f32; 2],
        size: [f32; 2],
        defaults: &HudLayout,
    ) -> Option<(Vec<HudLayout>, usize)> {
        match phase {
            TouchPhase::Started => return self.press(id, at, size, defaults),
            TouchPhase::Moved => {
                if let (Some((finger, offset)), Some(sel)) = (self.drag, self.selected)
                    && finger == id
                {
                    let [w, h] = size;
                    let (x, y) = ((at[0] + offset[0]) / w, (at[1] + offset[1]) / h);
                    let layout = self.current_mut();
                    match sel {
                        Sel::Button(i) => {
                            if let Some(b) = layout.buttons.get_mut(i) {
                                b.x = x.clamp(0.0, 1.0);
                                b.y = y.clamp(0.0, 1.0);
                            }
                        }
                        Sel::Stick => {
                            let [_, _, sw, sh] = layout.stick;
                            layout.stick[0] = x.clamp(0.0, 1.0 - sw);
                            layout.stick[1] = y.clamp(0.0, 1.0 - sh);
                        }
                        Sel::Esc => layout.esc = [x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)],
                    }
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if self.drag.is_some_and(|(finger, _)| finger == id) {
                    self.drag = None;
                }
            }
        }
        None
    }

    fn press(
        &mut self,
        id: u64,
        at: [f32; 2],
        size: [f32; 2],
        defaults: &HudLayout,
    ) -> Option<(Vec<HudLayout>, usize)> {
        if self.picking {
            let (panel, tiles) = picker(size, &self.current().buttons);
            if let Some(&(action, _)) = tiles.iter().find(|(_, r)| inside(*r, at)) {
                let buttons = &mut self.current_mut().buttons;
                buttons.push(Placed {
                    id: ACTIONS[action].id.to_owned(),
                    x: 0.5,
                    y: 0.5,
                    size: NEW_SIZE,
                });
                self.selected = Some(Sel::Button(buttons.len() - 1));
                self.picking = false;
            } else if !inside(panel, at) {
                self.picking = false;
            }
            return None;
        }
        // What the player placed comes before the tools: dragged over them, it can always be
        // dragged off again.
        let [w, h] = size;
        let layout = self.current();
        let hit = if inside(layout.esc_rect(size), at) {
            Some(Sel::Esc)
        } else if let Some(i) = layout.buttons.iter().rposition(|b| inside(b.rect(size), at)) {
            Some(Sel::Button(i))
        } else if inside(layout.stick_rect(size), at) {
            Some(Sel::Stick)
        } else {
            None
        };
        // Where the finger holds it: from the centre, the stick area from its corner.
        let grip = match hit {
            Some(Sel::Button(i)) => {
                let [x, y, bw, bh] = layout.buttons[i].rect(size);
                Some([x + bw / 2.0 - at[0], y + bh / 2.0 - at[1]])
            }
            Some(Sel::Esc) => Some([layout.esc[0] * w - at[0], layout.esc[1] * h - at[1]]),
            Some(Sel::Stick) => Some([layout.stick[0] * w - at[0], layout.stick[1] * h - at[1]]),
            None => None,
        };
        if hit.is_none()
            && let Some(&(tool, _, _)) = toolbar(size).iter().find(|(_, _, r)| inside(*r, at))
        {
            return self.tool(tool, defaults);
        }
        self.selected = hit;
        self.drag = grip.map(|g| (id, g));
        None
    }

    fn tool(&mut self, tool: Tool, defaults: &HudLayout) -> Option<(Vec<HudLayout>, usize)> {
        let selected = self.selected;
        let layout = self.current_mut();
        match tool {
            Tool::Add => self.picking = true,
            Tool::Remove => {
                if let Some(Sel::Button(i)) = selected {
                    layout.buttons.remove(i);
                    self.selected = None;
                }
            }
            Tool::Smaller | Tool::Bigger => {
                let factor = if tool == Tool::Bigger { 1.15 } else { 1.0 / 1.15 };
                match selected {
                    Some(Sel::Button(i)) => {
                        if let Some(b) = layout.buttons.get_mut(i) {
                            b.size = (b.size * factor).clamp(MIN_SIZE, MAX_SIZE);
                        }
                    }
                    Some(Sel::Stick) => {
                        let [x, y, sw, sh] = layout.stick;
                        let (cx, cy) = (x + sw / 2.0, y + sh / 2.0);
                        let (sw, sh) = ((sw * factor).clamp(0.15, 0.8), (sh * factor).clamp(0.2, 1.0));
                        layout.stick = [
                            (cx - sw / 2.0).clamp(0.0, 1.0 - sw),
                            (cy - sh / 2.0).clamp(0.0, 1.0 - sh),
                            sw,
                            sh,
                        ];
                    }
                    Some(Sel::Esc) | None => {}
                }
            }
            Tool::Fainter | Tool::Bolder => {
                let step = if tool == Tool::Bolder { 0.15 } else { -0.15 };
                layout.opacity = (layout.opacity + step).clamp(MIN_OPACITY, 1.0);
            }
            Tool::Layout => {
                self.active = (self.active + 1) % self.layouts.len().max(1);
                self.selected = None;
            }
            Tool::Reset => {
                *layout = defaults.clone();
                self.selected = None;
            }
            Tool::Done => return Some((self.layouts.clone(), self.active)),
        }
        None
    }
}

/// The layouts to edit: the saved ones, and the defaults in the empty slots.
pub(super) fn slots(saved: &[Option<HudLayout>], defaults: &HudLayout) -> Vec<HudLayout> {
    (0..LAYOUTS)
        .map(|i| {
            saved
                .get(i)
                .cloned()
                .flatten()
                .unwrap_or_else(|| defaults.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: [f32; 2] = [2250.0, 1143.0];

    fn centre(r: [f32; 4]) -> [f32; 2] {
        [r[0] + r[2] / 2.0, r[1] + r[3] / 2.0]
    }
    fn tool(t: Tool) -> [f32; 2] {
        centre(toolbar(SIZE).iter().find(|(x, _, _)| *x == t).unwrap().2)
    }
    fn button(id: &str, x: f32, y: f32) -> Placed {
        Placed {
            id: id.into(),
            x,
            y,
            size: 0.15,
        }
    }
    fn layout(buttons: Vec<Placed>) -> HudLayout {
        HudLayout {
            buttons,
            stick: [0.0, 0.3, 0.4, 0.7],
            esc: [0.5, 0.05],
            opacity: 1.0,
        }
    }
    fn editor(buttons: Vec<Placed>) -> Editor {
        Editor::new(slots(&[], &layout(buttons)), 0)
    }

    #[test]
    fn the_toolbar_and_the_picker_fit_and_show_what_is_missing() {
        for size in [SIZE, [1334.0, 750.0], [2048.0, 1536.0], [1536.0, 2048.0]] {
            let bar = toolbar(size);
            assert_eq!(bar.len(), TOOLS.len());
            for (i, (_, _, r)) in bar.iter().enumerate() {
                assert!(r[0] >= 0.0 && r[0] + r[2] <= size[0], "{r:?} {size:?}");
                for (_, _, o) in &bar[i + 1..] {
                    let apart = r[0] + r[2] <= o[0] || o[0] + o[2] <= r[0] || r[1] + r[3] <= o[1] || o[1] + o[3] <= r[1];
                    assert!(apart, "{r:?} {o:?} {size:?}");
                }
            }
            let (panel, tiles) = picker(size, &[]);
            assert_eq!(tiles.len(), ACTIONS.len());
            for (_, r) in &tiles {
                assert!(r[0] >= panel[0] && r[0] + r[2] <= panel[0] + panel[2], "{r:?} {size:?}");
                assert!(r[1] + r[3] <= panel[1] + panel[3], "{r:?} {size:?}");
            }
        }
        let (_, tiles) = picker(SIZE, &[button("jump", 0.5, 0.5)]);
        assert_eq!(tiles.len(), ACTIONS.len() - 1);
        assert!(tiles.iter().all(|(i, _)| ACTIONS[*i].id != "jump"));
    }

    #[test]
    fn drag_moves_the_button_under_the_finger() {
        let mut e = editor(vec![button("jump", 0.5, 0.5)]);
        let d = e.current().clone();
        let at = [SIZE[0] * 0.5 + 10.0, SIZE[1] * 0.5];
        assert!(e.touch(1, TouchPhase::Started, at, SIZE, &d).is_none());
        assert_eq!(e.selected, Some(Sel::Button(0)));
        e.touch(1, TouchPhase::Moved, [at[0] + 225.0, at[1]], SIZE, &d);
        assert!((e.current().buttons[0].x - 0.6).abs() < 1e-4, "{}", e.current().buttons[0].x);
        e.touch(1, TouchPhase::Ended, [at[0] + 225.0, at[1]], SIZE, &d);
        // Another finger later does not move it.
        e.touch(2, TouchPhase::Moved, [0.0, 0.0], SIZE, &d);
        assert!((e.current().buttons[0].x - 0.6).abs() < 1e-4);
    }

    #[test]
    fn the_stick_area_and_esc_move_and_the_area_resizes() {
        let mut e = editor(vec![]);
        let d = e.current().clone();
        // The stick area: grabbed inside, moved right, kept on the screen.
        let at = [SIZE[0] * 0.1, SIZE[1] * 0.6];
        e.touch(1, TouchPhase::Started, at, SIZE, &d);
        assert_eq!(e.selected, Some(Sel::Stick));
        e.touch(1, TouchPhase::Moved, [at[0] + SIZE[0] * 0.2, at[1]], SIZE, &d);
        assert!((e.current().stick[0] - 0.2).abs() < 1e-4);
        e.touch(1, TouchPhase::Moved, [at[0] + SIZE[0], at[1]], SIZE, &d);
        assert!((e.current().stick[0] - 0.6).abs() < 1e-4);
        e.touch(1, TouchPhase::Ended, at, SIZE, &d);
        e.touch(1, TouchPhase::Started, tool(Tool::Smaller), SIZE, &d);
        assert!(e.current().stick[2] < 0.4);
        // Esc.
        let esc = centre(e.current().esc_rect(SIZE));
        e.touch(2, TouchPhase::Started, esc, SIZE, &d);
        assert_eq!(e.selected, Some(Sel::Esc));
        e.touch(2, TouchPhase::Moved, [esc[0], esc[1] + SIZE[1] * 0.5], SIZE, &d);
        assert!((e.current().esc[1] - 0.55).abs() < 1e-3, "{:?}", e.current().esc);
    }

    #[test]
    fn add_remove_resize_opacity_reset_and_done() {
        let defaults = layout(vec![button("jump", 0.9, 0.9)]);
        let mut e = Editor::new(slots(&[], &defaults), 0);
        e.touch(1, TouchPhase::Started, tool(Tool::Add), SIZE, &defaults);
        assert!(e.picking);
        let (_, tiles) = picker(SIZE, &e.current().buttons);
        let (action, rect) = tiles[0];
        e.touch(1, TouchPhase::Started, centre(rect), SIZE, &defaults);
        assert!(!e.picking);
        assert_eq!(e.current().buttons.len(), 2);
        assert_eq!(e.current().buttons[1].id, ACTIONS[action].id);
        assert_eq!(e.selected, Some(Sel::Button(1)));
        e.touch(1, TouchPhase::Started, tool(Tool::Bigger), SIZE, &defaults);
        assert!(e.current().buttons[1].size > NEW_SIZE);
        for _ in 0..30 {
            e.touch(1, TouchPhase::Started, tool(Tool::Smaller), SIZE, &defaults);
        }
        assert!((e.current().buttons[1].size - MIN_SIZE).abs() < 1e-6);
        e.touch(1, TouchPhase::Started, tool(Tool::Remove), SIZE, &defaults);
        assert_eq!(e.current().buttons.len(), 1);
        assert_eq!(e.selected, None);
        for _ in 0..10 {
            e.touch(1, TouchPhase::Started, tool(Tool::Fainter), SIZE, &defaults);
        }
        assert!((e.current().opacity - MIN_OPACITY).abs() < 1e-6);
        e.touch(1, TouchPhase::Started, tool(Tool::Reset), SIZE, &defaults);
        assert_eq!(e.current(), &defaults);
        let done = e.touch(1, TouchPhase::Started, tool(Tool::Done), SIZE, &defaults);
        assert_eq!(done, Some((vec![defaults.clone(); LAYOUTS], 0)));
    }

    #[test]
    fn layouts_are_kept_apart_and_done_picks_the_one_shown() {
        let defaults = layout(vec![button("jump", 0.9, 0.9)]);
        let mut e = Editor::new(slots(&[], &defaults), 0);
        e.touch(1, TouchPhase::Started, tool(Tool::Layout), SIZE, &defaults);
        assert_eq!(e.active, 1);
        e.touch(1, TouchPhase::Started, tool(Tool::Fainter), SIZE, &defaults);
        e.touch(1, TouchPhase::Started, tool(Tool::Layout), SIZE, &defaults);
        e.touch(1, TouchPhase::Started, tool(Tool::Layout), SIZE, &defaults);
        assert_eq!(e.active, 0);
        assert_eq!(e.current(), &defaults);
        e.touch(1, TouchPhase::Started, tool(Tool::Layout), SIZE, &defaults);
        let (layouts, active) = e
            .touch(1, TouchPhase::Started, tool(Tool::Done), SIZE, &defaults)
            .unwrap();
        assert_eq!(active, 1);
        assert!(layouts[1].opacity < 1.0);
        assert_eq!(layouts[0], defaults);
    }

    #[test]
    fn a_tap_outside_the_picker_closes_it_and_a_tap_on_nothing_deselects() {
        let mut e = editor(vec![button("jump", 0.5, 0.5)]);
        let d = e.current().clone();
        e.selected = Some(Sel::Button(0));
        e.touch(1, TouchPhase::Started, tool(Tool::Add), SIZE, &d);
        e.touch(1, TouchPhase::Started, [SIZE[0] - 5.0, SIZE[1] - 5.0], SIZE, &d);
        assert!(!e.picking);
        assert_eq!(e.current().buttons.len(), 1);
        e.touch(1, TouchPhase::Started, [SIZE[0] - 5.0, SIZE[1] - 5.0], SIZE, &d);
        assert_eq!(e.selected, None);
    }
}
