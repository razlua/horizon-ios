# Changelog

Ferrite's changes, newest first. **Unreleased** is what changed since the latest release and
becomes the next release's notes. Android builds are listed on the
[releases page](https://github.com/razlua/horizon-ios/releases). Upstream Horizon's own notes are
at [horizon-rust/horizon releases](https://github.com/horizon-rust/horizon/releases).

## Unreleased

### Android: the download says what it's doing

- **The game files' download shows each step and each retry.** It used to show only "..." until
  a whole megabyte had arrived, so a slow or retried first download looked stuck. Now the screen
  updates at every step and every 100 KB, shows a file's size as soon as the download starts, and
  shows "trying again (2 of 4: why)" when a download is retried.

### Android: mouse support (`android-port` branch)

- **A mouse works like on a computer.** On a Chromebook, or with a mouse plugged into a phone or
  tablet, moving the mouse turns the view while you play (the pointer is locked to the game, as
  on PC), left click breaks, right click places, the wheel scrolls the hotbar, and in menus the
  pointer clicks buttons. Before, Android sent the mouse as a finger. A keyboard already worked:
  WASD, Space, Shift, E, T, 1-9 and Esc. Touch controls work the same as before.

### Android 2026.11.12 (pre-release, `android-port` branch)

- **An Android port.** The game builds as an `.apk` for Android 10 and later, for phones and
  tablets (`arm64-v8a`) and for Android on PCs, Chromebooks and emulators (`x86_64`). It has the
  same touch HUD, Edit HUD, start screen with Microsoft sign-in, on-device download of the game
  files, Extra Settings and voice chat as the iOS app. The system's Back is Esc. See
  [docs/android.md](docs/android.md).

### Added

- **Launcher.** Ferrite opens on a native launcher with four tabs. Play has your player in 3D
  (drag to turn it), your accounts, a skin changer with a 3D preview, Microsoft sign-in and the
  download. Refresh shows when the app's signature runs out, with a reminder a day before and on
  the day. Settings has render distance, max frame rate, Battery mode and Extra Settings. About has
  the credits and Send logs.
- **Refresh now, inside Ferrite.** Sign in with your Apple ID on the Refresh tab and Ferrite signs
  itself again for 7 days and installs over itself, worlds and sign-ins kept, with no computer and
  no SideStore. It needs LocalDevVPN (free, App Store) on and a pairing file made once on a
  computer. It also asks Apple for the increased memory limit, for higher render distances.
  Experimental: please send the log if it fails.
- **Speed Trade.** A >> button next to a trade's result keeps making the selected trade until you
  stop it or the villager runs out. Speed (4, 10 or 20 a second) and Hold or Tap are in Extra
  Settings. Works in single player and on servers.
- **Controllers, keyboards and mice on iPhone and iPad.** Xbox, PlayStation, MFi and Switch Pro
  controllers work through Apple's GameController framework, with Horizon's own controller
  support on top: menus, the right button pictures, your bindings. A hardware keyboard plays as on
  a computer (WASD, E, F3, chat, mod keys), and a mouse turns the view and attacks and uses in the
  world. The touch buttons step aside while one of them plays and come back at the next touch.
  Experimental: please say how it feels.
- **Profiles.** The Play tab's profile menu keeps several sets of settings, for example one for PvP
  and one for survival: each has its own options, keybinds, video settings, servers list and
  offline name, and Settings changes the chosen one. Worlds, packs, mods and sign-ins stay shared.
  A new profile starts as a copy of your settings, and Default is everything as before.
- **Close.** Beside Extra Settings on the main menu, and Close Ferrite in the launcher's More menu:
  Ferrite ends without the app switcher.
- **Crash catcher.** If Ferrite closes by itself while you play (usually iOS ending it for memory,
  which leaves nothing in the log), the next start says so on the Play tab, with the render
  distance and memory left at the time and a guess at why. Send report shares it with the end of
  that run's log.
- **Feather.** The AltStore / SideStore source works in Feather too.
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
