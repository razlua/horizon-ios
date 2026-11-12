//! Simple Voice Chat (an unofficial client: crates/voicechat-engine) wired into the game's
//! hook points, as Horizon's voicechat native mod wires it on a computer: the menu (V), its keys,
//! the HUD status, `/vc`, its row and settings page in Options > Mods. On iOS, also the Edit HUD
//! buttons (Voice chat menu, Talk, Mic) and the menu's fields on the iPhone keyboard. The voices
//! themselves and the server's payloads are `remote::tick_voice_chat` and `handle_packet`.
use super::Game;
use super::native_hooks::{NativeMod, NativeScreenView};
use horizon_client::extras::Corner;
use horizon_client::extras_hud::Corners;
use horizon_client::hud::HudBatch;
use voicechat_engine as vc;

/// The touch buttons' key names (`touch::ACTIONS`), taken here instead of the keyboard's.
pub(super) const TOUCH_MENU: &str = "vc.menu";
pub(super) const TOUCH_TALK: &str = "vc.talk";
pub(super) const TOUCH_MUTE: &str = "vc.mute";

impl Game<'_> {
    fn vc_load(&self) {
        vc::load(&|k| self.native_setting(vc::ID, k));
    }

    fn vc_store(&self) {
        for (k, v) in vc::take_store() {
            self.native_set_setting(vc::ID, &k, &v);
        }
    }

    /// A key bound to the voice chat, or what the touch buttons press instead.
    fn vc_key_label(&self, name: &str, default: &str) -> String {
        if cfg!(any(target_os = "ios", target_os = "android")) {
            match name {
                vc::KEY_MENU => return "VC button".into(),
                vc::KEY_PTT => return "Talk button".into(),
                vc::KEY_MUTE => return "Mic button".into(),
                _ => {}
            }
        }
        vc::key_label(&self.native_mod_key(vc::ID, name, default))
    }

    /// Opens the voice chat menu, or closes it when it is open.
    fn vc_toggle_menu(&mut self, opened_by: Option<char>) {
        if self.native_mod_screen() == Some(vc::ID) {
            self.native_mod_screen_close(vc::ID);
            return;
        }
        if !self.native_playing() {
            return;
        }
        vc::menu_open(opened_by);
        if !self.native_mod_screen_open(vc::ID) {
            vc::menu_sync(false);
        }
    }

    /// One of the voice chat's keys going down or up.
    fn vc_press(&mut self, which: vc::Key, pressed: bool) {
        if pressed && !self.native_playing() {
            return;
        }
        self.vc_load();
        if let Some(line) = vc::key(which, pressed) {
            self.native_chat(&line);
        }
        self.vc_store();
    }

    /// The menu key (V) opens the menu; push to talk and whisper while held; mute and voice
    /// chat off/on on a press. A key goes down only while playing, but letting go always counts.
    pub(super) fn vc_key(&mut self, key: &str, pressed: bool) {
        if !self.native_on(vc::ID) {
            return;
        }
        let menu = self.native_mod_key(vc::ID, vc::KEY_MENU, vc::DEFAULT_MENU);
        if pressed && key == menu && menu != "key.keyboard.unknown" {
            if self.native_mod_screen() != Some(vc::ID) {
                self.vc_toggle_menu(vc::key_char(key));
            }
            return;
        }
        let bound = [
            (vc::Key::PushToTalk, vc::KEY_PTT, vc::DEFAULT_PTT),
            (vc::Key::Whisper, vc::KEY_WHISPER, vc::DEFAULT_WHISPER),
            (vc::Key::Mute, vc::KEY_MUTE, vc::DEFAULT_MUTE),
            (vc::Key::Disable, vc::KEY_DISABLE, vc::DEFAULT_DISABLE),
        ];
        let Some(which) = bound.into_iter().find_map(|(which, name, default)| {
            let chosen = self.native_mod_key(vc::ID, name, default);
            (chosen == key && chosen != "key.keyboard.unknown").then_some(which)
        }) else {
            return;
        };
        self.vc_press(which, pressed);
    }

    /// iOS: an Edit HUD button of the voice chat's (`TOUCH_*`). True when `name` was one.
    pub(super) fn vc_touch(&mut self, name: &str, down: bool) -> bool {
        // Android asks for the microphone at run time: when a voice chat button is first used.
        #[cfg(target_os = "android")]
        if down && matches!(name, TOUCH_MENU | TOUCH_TALK | TOUCH_MUTE) {
            horizon_threads::android_sys::request_microphone();
        }
        let which = match name {
            TOUCH_MENU => {
                if down && self.native_on(vc::ID) {
                    self.vc_toggle_menu(None);
                }
                return true;
            }
            TOUCH_TALK => vc::Key::PushToTalk,
            TOUCH_MUTE => vc::Key::Mute,
            _ => return false,
        };
        if self.native_on(vc::ID) {
            self.vc_press(which, down);
        }
        true
    }

    /// Each frame: the menu closes when it asked to, and forgets a close the game did (Escape,
    /// another screen).
    pub(super) fn vc_frame(&mut self) {
        if vc::menu_take_close() {
            self.native_mod_screen_close(vc::ID);
        }
        vc::menu_sync(self.native_mod_screen() == Some(vc::ID));
    }

    /// Whether the menu's focused field takes typing (the iPhone keyboard comes up for it).
    pub(super) fn vc_typing(&self) -> bool {
        self.native_mod_screen() == Some(vc::ID) && vc::menu_typing()
    }

    /// The status in the top left corner (connecting, talking, muted, the group and who is
    /// talking), and each player's group above their head.
    pub(super) fn vc_hud(&mut self, batch: &mut HudBatch, corners: &mut Corners) {
        if !self.native_on(vc::ID) {
            return;
        }
        if let Some((pos, yaw, pitch, fov)) = self.locator_camera {
            let camera = vc::labels::Camera { pos, yaw, pitch, fov };
            for label in vc::group_labels(&camera, [batch.width, batch.height]) {
                let w = self.native_text_width(&label.text) + 4.0;
                let [x, y] = [label.at[0] - w / 2.0, label.at[1] - 6.0];
                self.native_fill(batch, [x, y, w, 12.0], 0x6000_0000);
                self.native_text(batch, &label.text, [x + 2.0, y + 2.0], label.rgb, true);
            }
        }
        if !self.native_corner_shown(Corner::TopLeft) {
            return;
        }
        let ptt = if cfg!(any(target_os = "ios", target_os = "android")) {
            "key.keyboard.talk.button".to_owned()
        } else {
            self.native_mod_key(vc::ID, vc::KEY_PTT, vc::DEFAULT_PTT)
        };
        for line in vc::hud(&ptt) {
            let w = self.native_text_width(&line.text) + 4.0;
            let (x, y) = corners.place(Corner::TopLeft, w, 12.0);
            self.native_fill(batch, [x, y, w, 12.0], 0x9000_0000);
            self.native_text(batch, &line.text, [x + 2.0, y + 2.0], line.rgb, true);
        }
    }

    /// Draws the menu: the connection, the controls, the hotkeys as bound now, and the groups.
    pub(super) fn vc_screen(&self, batch: &mut HudBatch, screen: &NativeScreenView<'_>) {
        if self.native_mod_screen() != Some(vc::ID) {
            return;
        }
        let hotkeys = vc::HOTKEYS
            .iter()
            .map(|(name, what, default)| (what.to_string(), self.vc_key_label(name, default)))
            .collect();
        let measure = |t: &str| self.native_text_width(t);
        for draw in vc::menu_draw([batch.width, batch.height], screen.mouse, &measure, hotkeys) {
            match draw {
                vc::ui::Draw::Fill(rect, argb) => self.native_fill(batch, rect, argb),
                vc::ui::Draw::Text(text, at, rgb, shadow) => {
                    self.native_text(batch, &text, at, rgb, shadow)
                }
            }
        }
    }

    /// A click on the menu: the button or field under the pointer.
    pub(super) fn vc_mouse(&self, screen: &NativeScreenView<'_>, button: i32, canceled: &mut bool) {
        if self.native_mod_screen() != Some(vc::ID) {
            return;
        }
        *canceled = true;
        if button == 0 {
            vc::menu_click(screen.mouse);
            self.vc_store();
        }
    }

    /// The wheel (a swipe) over the menu scrolls the groups.
    pub(super) fn vc_scroll(&self, scroll: [f64; 2], canceled: &mut bool) {
        if self.native_mod_screen() != Some(vc::ID) {
            return;
        }
        *canceled = true;
        vc::menu_scroll(scroll[1]);
    }

    /// Keys over the menu are the menu's: typing, Tab, Enter, Escape, and the menu key closes it.
    pub(super) fn vc_keys(&self, key: &str, modifiers: u32, canceled: &mut bool) {
        if self.native_mod_screen() != Some(vc::ID) {
            return;
        }
        *canceled = true;
        let menu = self.native_mod_key(vc::ID, vc::KEY_MENU, vc::DEFAULT_MENU);
        vc::menu_key(key, modifiers & 1 != 0, &menu);
        self.vc_store();
    }

    /// Characters typed on the menu go into the focused field.
    pub(super) fn vc_chars(&self, c: char, canceled: &mut bool) {
        if self.native_mod_screen() != Some(vc::ID) {
            return;
        }
        *canceled = true;
        vc::menu_char(c);
    }

    /// `/vc ...`, answered in the chat.
    pub(super) fn vc_command(&mut self, command: &str, ran: &mut bool) {
        if *ran || !self.native_on(vc::ID) {
            return;
        }
        self.vc_load();
        let Some(lines) = vc::command(command) else {
            return;
        };
        *ran = true;
        for line in lines {
            self.native_chat(&line);
        }
        self.vc_store();
    }

    /// `/vc` completes and colours as the game's own commands do.
    pub(super) fn vc_commands(commands: &mut Vec<&'static str>) {
        commands.push(vc::COMMAND);
    }

    /// The voice chat's row in Options > Mods, with its keys.
    pub(super) fn vc_mods(mods: &mut Vec<NativeMod>) {
        mods.push(NativeMod {
            id: vc::ID.into(),
            title: format!("{} {}", vc::NAME, vc::VERSION),
            description: vc::DESCRIPTION.into(),
            keys: vc::HOTKEYS
                .iter()
                .map(|(name, what, default)| {
                    [name.to_string(), what.to_string(), default.to_string()]
                })
                .collect(),
        });
    }

    /// Its settings page: activation, microphone, volumes and the protocol.
    pub(super) fn vc_settings(&mut self, mod_id: &str, buttons: &mut Vec<String>) {
        if mod_id != vc::ID {
            return;
        }
        self.vc_load();
        buttons.extend(vc::settings_buttons());
    }

    /// A click on its settings page: the button's setting moves to its next value.
    pub(super) fn vc_settings_click(&mut self, mod_id: &str, button: usize) {
        if mod_id != vc::ID {
            return;
        }
        self.vc_load();
        vc::settings_click(button);
        self.vc_store();
    }
}
