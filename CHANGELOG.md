# Changelog: Horizon for iOS

The iOS port's changes, newest first. Upstream Horizon's own release notes are at
[horizon-rust/horizon releases](https://github.com/horizon-rust/horizon/releases).

## Next

### Added

- **Offline name.** **Play offline** on the start screen lets you choose your name (3 to 16
  letters, numbers or _) for single player and offline-mode ("cracked") servers.
- **About the Project** in Options, under Mods and right of Credits & Attribution: who makes the
  port (@4rnz on Discord, razlua on GitHub) and where to report bugs.
- **Esc resizes** in Edit HUD: Smaller and Bigger work on it as on the other buttons.
- **Done higher.** Settings screens' bottom rows (Done, Cancel) sit a little higher on iPhone.

- **No computer needed.** The 30 MB app downloads the game files itself after a Microsoft sign-in:
  your Minecraft 26.3 from Mojang (about 520 MB), checked against Mojang's checksums, and
  Horizon's tables from this repository's `tables-6475f62` release (no Minecraft files in it),
  checked against the SHA-256 in the app. A progress bar shows the download, the screen stays on,
  and an interrupted download goes on where it stopped. The result is the same as a computer's
  preparation, file for file (all 28,166 files compared).

## 2026-11-11

### Removed

- **Fainter and Bolder** in Edit HUD: they took up the edit screen.
- **Safe areas:** the game draws edge to edge again.

### Added

- **Simple Voice Chat, listening.** On servers with Simple Voice Chat you hear the players who
  use it, from where they stand. The engine is Horizon's voicechat native mod's (an unofficial
  client), built into the app with Opus 1.5.2 linked in. The microphone is never opened.
- **Accounts.** Several Microsoft accounts. The start screen lists them before the game: tap one
  to play as it, **Add account**, **Remove an account**, or **Play offline**. An account saved by
  an earlier build carries over.
- **Chat buttons.** Over chat, at the top right: **Tab** (completes commands and names),
  **Paste** and **Enter** (sends). Tapping chat's line brings the keyboard back.
- **Clipboard.** Copy and paste go through the iPhone's clipboard (`UIPasteboard`), so they work
  with other apps.
- **Unpacking progress.** A progress bar on the start screen while the first launch unpacks.
- **Edit HUD extras.** The move stick's area can be moved and resized, and the Esc button
  moved. **Layout** switches between three saved layouts. The toolbar moved below the top row of
  buttons, and buttons over it can be dragged off.

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
