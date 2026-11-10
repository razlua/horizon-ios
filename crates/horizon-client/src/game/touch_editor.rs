//! Edit HUD (the iOS build's title, in Quit Game's place): the player moves, adds, removes and
//! resizes the touch buttons (touch.rs), and the layout is kept in `touch-controls.json` in the
//! game folder.
//!
//! The logic is plain data in and out, as in touch.rs: `Editor::touch` takes a finger in window
//! pixels and says when the player is done.

use super::touch::{ACTIONS, Placed};
use winit::event::TouchPhase;

/// The editor's own buttons, along the top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tool {
    Add,
    Remove,
    Smaller,
    Bigger,
    Reset,
    Done,
}

pub(super) const TOOLS: [(Tool, &str); 6] = [
    (Tool::Add, "Add"),
    (Tool::Remove, "Remove"),
    (Tool::Smaller, "Smaller"),
    (Tool::Bigger, "Bigger"),
    (Tool::Reset, "Reset"),
    (Tool::Done, "Done"),
];

/// A button's size for one just added, a share of the screen's short side.
const NEW_SIZE: f32 = 0.11;
const MIN_SIZE: f32 = 0.06;
const MAX_SIZE: f32 = 0.30;

fn inside([x, y, w, h]: [f32; 4], [px, py]: [f32; 2]) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}

/// The unit the editor's own parts are measured in: a share of the short side.
fn unit([w, h]: [f32; 2]) -> f32 {
    w.min(h) * 0.15
}

/// The tools' rectangles along the top, centred.
pub(super) fn toolbar(size: [f32; 2]) -> Vec<(Tool, &'static str, [f32; 4])> {
    let u = unit(size);
    let (bw, bh, gap) = (u * 1.15, u * 0.5, u * 0.12);
    let total = TOOLS.len() as f32 * bw + (TOOLS.len() - 1) as f32 * gap;
    let mut x = (size[0] - total) / 2.0;
    TOOLS
        .iter()
        .map(|&(tool, label)| {
            let rect = [x, u * 0.12, bw, bh];
            x += bw + gap;
            (tool, label, rect)
        })
        .collect()
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

/// The layout being edited.
pub(super) struct Editor {
    pub buttons: Vec<Placed>,
    pub selected: Option<usize>,
    /// The Add panel is open.
    pub picking: bool,
    /// The finger moving a button, and where it holds it (from the button's centre).
    drag: Option<(u64, [f32; 2])>,
}

impl Editor {
    pub(super) fn new(buttons: Vec<Placed>) -> Self {
        Self {
            buttons,
            selected: None,
            picking: false,
            drag: None,
        }
    }

    /// One finger. `defaults` is the layout Reset puts back. The finished layout when the player
    /// taps Done.
    pub(super) fn touch(
        &mut self,
        id: u64,
        phase: TouchPhase,
        at: [f32; 2],
        size: [f32; 2],
        defaults: &[Placed],
    ) -> Option<Vec<Placed>> {
        match phase {
            TouchPhase::Started => return self.press(id, at, size, defaults),
            TouchPhase::Moved => {
                if let (Some((finger, offset)), Some(i)) = (self.drag, self.selected)
                    && finger == id
                    && let Some(b) = self.buttons.get_mut(i)
                {
                    b.x = ((at[0] + offset[0]) / size[0]).clamp(0.0, 1.0);
                    b.y = ((at[1] + offset[1]) / size[1]).clamp(0.0, 1.0);
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
        defaults: &[Placed],
    ) -> Option<Vec<Placed>> {
        if self.picking {
            let (panel, tiles) = picker(size, &self.buttons);
            if let Some(&(action, _)) = tiles.iter().find(|(_, r)| inside(*r, at)) {
                self.buttons.push(Placed {
                    id: ACTIONS[action].id.to_owned(),
                    x: 0.5,
                    y: 0.5,
                    size: NEW_SIZE,
                });
                self.selected = Some(self.buttons.len() - 1);
                self.picking = false;
            } else if !inside(panel, at) {
                self.picking = false;
            }
            return None;
        }
        if let Some(&(tool, _, _)) = toolbar(size).iter().find(|(_, _, r)| inside(*r, at)) {
            match tool {
                Tool::Add => self.picking = true,
                Tool::Remove => {
                    if let Some(i) = self.selected.take() {
                        self.buttons.remove(i);
                    }
                }
                Tool::Smaller | Tool::Bigger => {
                    if let Some(b) = self.selected.and_then(|i| self.buttons.get_mut(i)) {
                        let factor = if tool == Tool::Bigger { 1.15 } else { 1.0 / 1.15 };
                        b.size = (b.size * factor).clamp(MIN_SIZE, MAX_SIZE);
                    }
                }
                Tool::Reset => {
                    self.buttons = defaults.to_vec();
                    self.selected = None;
                }
                Tool::Done => return Some(self.buttons.clone()),
            }
            return None;
        }
        // The top-most button under the finger: the last one drawn.
        let hit = self
            .buttons
            .iter()
            .rposition(|b| inside(b.rect(size), at));
        self.selected = hit;
        self.drag = hit.map(|i| {
            let [x, y, w, h] = self.buttons[i].rect(size);
            (id, [x + w / 2.0 - at[0], y + h / 2.0 - at[1]])
        });
        None
    }
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

    #[test]
    fn the_toolbar_and_the_picker_fit_and_show_what_is_missing() {
        for size in [SIZE, [1334.0, 750.0], [2048.0, 1536.0]] {
            let bar = toolbar(size);
            assert!(bar.first().unwrap().2[0] >= 0.0);
            let last = bar.last().unwrap().2;
            assert!(last[0] + last[2] <= size[0]);
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
        let mut e = Editor::new(vec![button("jump", 0.5, 0.5)]);
        let at = [SIZE[0] * 0.5 + 10.0, SIZE[1] * 0.5];
        assert!(e.touch(1, TouchPhase::Started, at, SIZE, &[]).is_none());
        assert_eq!(e.selected, Some(0));
        e.touch(1, TouchPhase::Moved, [at[0] + 225.0, at[1]], SIZE, &[]);
        assert!((e.buttons[0].x - 0.6).abs() < 1e-4, "{}", e.buttons[0].x);
        e.touch(1, TouchPhase::Ended, [at[0] + 225.0, at[1]], SIZE, &[]);
        // Another finger later does not move it.
        e.touch(2, TouchPhase::Moved, [0.0, 0.0], SIZE, &[]);
        assert!((e.buttons[0].x - 0.6).abs() < 1e-4);
    }

    #[test]
    fn add_remove_resize_reset_and_done() {
        let defaults = vec![button("jump", 0.9, 0.9)];
        let mut e = Editor::new(defaults.clone());
        e.touch(1, TouchPhase::Started, tool(Tool::Add), SIZE, &defaults);
        assert!(e.picking);
        let (_, tiles) = picker(SIZE, &e.buttons);
        let (action, rect) = tiles[0];
        e.touch(1, TouchPhase::Started, centre(rect), SIZE, &defaults);
        assert!(!e.picking);
        assert_eq!(e.buttons.len(), 2);
        assert_eq!(e.buttons[1].id, ACTIONS[action].id);
        assert_eq!(e.selected, Some(1));
        e.touch(1, TouchPhase::Started, tool(Tool::Bigger), SIZE, &defaults);
        assert!(e.buttons[1].size > NEW_SIZE);
        for _ in 0..30 {
            e.touch(1, TouchPhase::Started, tool(Tool::Smaller), SIZE, &defaults);
        }
        assert!((e.buttons[1].size - MIN_SIZE).abs() < 1e-6);
        e.touch(1, TouchPhase::Started, tool(Tool::Remove), SIZE, &defaults);
        assert_eq!(e.buttons.len(), 1);
        assert_eq!(e.selected, None);
        e.buttons.clear();
        e.touch(1, TouchPhase::Started, tool(Tool::Reset), SIZE, &defaults);
        assert_eq!(e.buttons, defaults);
        let done = e.touch(1, TouchPhase::Started, tool(Tool::Done), SIZE, &defaults);
        assert_eq!(done, Some(defaults));
    }

    #[test]
    fn a_tap_outside_the_picker_closes_it_and_a_tap_on_nothing_deselects() {
        let mut e = Editor::new(vec![button("jump", 0.5, 0.5)]);
        e.selected = Some(0);
        e.touch(1, TouchPhase::Started, tool(Tool::Add), SIZE, &[]);
        e.touch(1, TouchPhase::Started, [5.0, SIZE[1] - 5.0], SIZE, &[]);
        assert!(!e.picking);
        assert_eq!(e.buttons.len(), 1);
        e.touch(1, TouchPhase::Started, [5.0, SIZE[1] - 5.0], SIZE, &[]);
        assert_eq!(e.selected, None);
    }
}
