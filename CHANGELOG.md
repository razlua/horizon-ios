# Changelog

Ferrite's changes, newest first. **Unreleased** is what changed since the latest release and
becomes the next release's notes. Android builds are listed on the
[releases page](https://github.com/razlua/horizon-ios/releases). Upstream Horizon's own notes are
at [horizon-rust/horizon releases](https://github.com/horizon-rust/horizon/releases).

## Unreleased

### Added

- **Launcher.** Ferrite opens on a native launcher with four tabs. Play has your player in 3D
  (drag to turn it), your accounts, a skin changer with a 3D preview, Microsoft sign-in and the
  download. Refresh shows when the app's
  signature runs out, with a reminder a day before and on the day, and lets you sign in with your
  Apple ID to check it. Settings has render distance, max frame rate, Battery mode and Extra Settings.
  About has the credits and Send logs.
- **Speed Trade.** A >> button next to a trade's result keeps making the selected trade until you
  stop it or the villager runs out. Speed (4, 10 or 20 a second) and Hold or Tap are in Extra
  Settings. Works in single player and on servers.
- **Inventory by finger.** Hold a stack and slide across slots to split it evenly, as a mouse
  drag does on PC. Hold still on the first slot before sliding to put one item in each. Lists
  (creative tabs, recipe book, JEI, packs) keep gliding after a quick swipe and slow down, and a
  touch stops them.
- **Ferrite VC.** The voice chat has a new name and a new menu with Voice, Groups and Keys tabs,
  drawn with the game's own buttons, sliders and text fields.

### Fixed

- The villager trading list scrolls when you drag it. A tap still picks a trade.
- The render distance no longer drops to 2 and stays there, and it no longer drops early: the
  memory guard steps down only when less than 200 MB is left, so an iPhone 17 keeps 29 instead of
  dropping to 17.
- Far chunks show up much sooner. A world still starts at render distance 8, but it now climbs a
  third of the way to your setting every 3 seconds (8 to 29 in about 20 seconds) instead of one
  step every 6 seconds (over 2 minutes).
- Big render distances no longer crash right after a world opens. A world starts at 8 and climbs
  to your setting while there's memory for it.
- The first download shows each step, its size and each retry, instead of "..." until a whole
  megabyte had arrived.

### Changed

- **The app is now called Ferrite**, with a new yellow icon. It installs over the old app with
  your worlds, sign-ins and settings, and updates come through the same source.

## 2026.11.13

### Added

- **One-tap updates with SideStore or AltStore.** When a newer version is out, the start screen
  offers Update with SideStore (or AltStore) if one is installed. Without either, the button opens
  the download page.

### Fixed

- High render distances no longer crash the app. The game lowers the distance it uses while
  memory is tight and raises it again when there's room. Your setting stays as you chose it.

### Changed

- Less CPU time per frame: the app is built fully optimized, with Horizon's faster memory
  allocator.
- Culling and caches are on by default: hidden entities, particles and inner leaf faces aren't
  drawn, and chests, signs and similar blocks keep their models while still. Each can be turned
  off in Video Settings.

## 2026.11.12-2

### Added

- **59 native mods** from Horizon's registry are built in, all off until you turn them on in
  *Options → Mods* ([NATIVE_MODS.md](NATIVE_MODS.md)).
- **Modrinth resource packs.** The Resource Packs screen's Add button searches Modrinth and
  installs a pack with a tap.
- **Key buttons for mods** in Edit HUD (B, C, E, G and more, plus Tab).

### Fixed

- The picture was stretched sideways by about 14%. It now draws at the screen's own size.
- Edit HUD's tools sit above the buttons and always take a tap.
- The keyboard didn't open for search boxes in Options opened from the main menu.
- Loading screens covered only part of the screen with Safe Area on.
- Open Pack Folder and the other folder buttons open the Files app.
- Long button names in Edit HUD's Add list fit their tiles.

### Changed

- Release notes are now taken from this changelog.
- The README has a Performance section with iPhone 17 screenshots.

## 2026.11.12

### Added

- **Talking in voice chat**, with VC, Talk and Mic buttons in Edit HUD. The microphone opens only
  when you first talk.
- **Update notice** on the start screen when a newer version is out.
- **Extra Settings** on the main menu: Safe Area, button opacity, touch look speed and keep screen
  on.
- **Chat above the keyboard** while you type.
- **AltStore and SideStore source**, so new builds arrive as updates.

## 2026.11.11-2

### Added

- **No computer needed.** The app downloads the game files itself after a Microsoft sign-in
  (about 520 MB from Mojang, checked against Mojang's checksums) and resumes an interrupted
  download.
- **Offline name** for single player and offline-mode servers.
- **About the Project** in Options, with who makes the port and where to report bugs.
- The Esc button can be resized in Edit HUD.

## 2026.11.11

### Added

- **Voice chat, listening.** On servers with Simple Voice Chat you hear the players who use it.
- **Several Microsoft accounts**, listed on the start screen.
- **Chat buttons:** Tab, Paste and Enter.
- **Clipboard** shared with other apps.
- **Edit HUD:** a movable move-stick area and Esc button, and three saved layouts.

### Removed

- Fainter and Bolder in Edit HUD.
- Safe areas (the game draws edge to edge again).

## First release (2026-11-10)

Based on upstream Horizon `6475f62`.

### Added

- Touch controls: a floating move stick, action buttons, look by dragging, and tap, scroll, drag
  and right-click in menus.
- Edit HUD, with hotbar slot, key and controller-action buttons.
- An Esc button at the top centre.
- A start screen that unpacks the game files and signs you in with Microsoft.
- 120 Hz on ProMotion screens.
- Pausing and saving when you leave the app.

### Fixed

- Chat didn't send from the iPhone keyboard.
- Servers behind an SRV record refused the connection.
- Improved transparency crashed on iPhone GPUs.
- A black screen, a zoomed-in screen and taps landing in the wrong place.
- The keyboard stayed open.
