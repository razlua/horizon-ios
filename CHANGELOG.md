# Changelog: Horizon for iOS

The iOS port's changes, newest first. Upstream Horizon's own release notes are at
[horizon-rust/horizon releases](https://github.com/horizon-rust/horizon/releases).

## 2026-11-10

All builds so far are based on upstream Horizon `6475f62`.

### Added

- **Hotbar slot buttons** 1 to 9 in Edit HUD: a tap selects that slot.
- **120 Hz** on ProMotion screens: the app asks iOS for the screen's highest frame rate (it stayed
  at 60 without asking).
- **Edit HUD**, on iOS in Quit Game's place on the main menu. You can move, add, remove and
  resize the touch buttons, and Reset brings back the defaults. Your layout is kept in
  `touch-controls.json`.
- **Key buttons**: F1, F2, F3, F3+B, F3+G, F3+H, F3+N, F3+T, F3+F4 and `/`.
- **More controller-action buttons**: sprint, swap hands, pick block, advancements and the
  player list.
- **An Esc button** at the top centre, in the world and in every screen. It replaces the old
  pause button.
- **The start screen**: it unpacks your game files on the first launch and signs you in with
  Microsoft (Safari opens with your code filled in). Later launches sign you in by themselves,
  and **Play offline** skips it.
- **Touch controls**: a floating move stick, action buttons, look by dragging, and tap, scroll,
  drag and right-click in menus.
- **Pausing and saving** when the app leaves the screen.
- **Diagnostics in the log**: what the app holds at startup, and how touches are scaled.

### Fixed

- **Chat did not send.** The iPhone keyboard's return key now sends the message (and runs
  commands, confirms signs and screens) as Enter does.
- **The controller's on-screen keyboard** opened over chat beside the iPhone's; it is off on iOS.
- Servers behind a DNS redirect (an SRV record, as most hosted servers use) refused the
  connection: an iOS app can't read `/etc/resolv.conf`, so the redirect was never looked up. It
  is now asked of the system's resolver.
- **Crash with improved transparency.** Turning it on ended the game on iOS GPUs, which can't
  blend the texture format it uses. On those GPUs the option now does nothing.
- **Black screen.** The renderer looked for Vulkan and OpenGL instead of Metal.
- **Zoomed-in screen.** The window was created at a desktop size, larger than the screen.
- **Taps landing in the wrong place.** Touches and the screen used different scales, so the
  action and menu buttons missed.
- **Keyboard stuck open.** The iPhone keyboard never closed. Return now confirms and closes it,
  and so does a tap outside the text box.
- **Missing game files.** The app now says plainly when it was installed without its game
  files.

### Build

- **Intel runners.** The iOS workflow runs on GitHub's Intel macOS runners, because the Apple
  Silicon ones stayed queued.
