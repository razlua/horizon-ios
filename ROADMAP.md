# Roadmap: Ferrite

This roadmap covers the iOS port only. Upstream Horizon's own roadmap is in
[docs/upstream/ROADMAP.md](docs/upstream/ROADMAP.md). The game itself (world generation, mobs,
redstone, multiplayer and so on) comes from upstream.

## Done

- [x] The game client builds for `aarch64-apple-ios` on GitHub's macOS runners and is packaged
      as an unsigned `.ipa`.
- [x] Drawing with Metal in a window that fills the screen, sideways only, at up to 120 Hz on
      ProMotion screens.
- [x] No computer needed for the game files. After a Microsoft sign-in the app downloads the
      player's own Minecraft from Mojang and combines it with Horizon's published tables (the part
      that needs Java, worked out once), as upstream issue #1750 proposes. The result matches a
      computer's preparation file for file.
- [x] Game files can also be added to the app on a computer, without uploading them anywhere. The
      app unpacks them on its first launch beside the event loop, so iOS doesn't kill a slow start.
- [x] A start screen from the first frame, with a progress bar during the first launch.
- [x] Microsoft sign-in without a launcher: a device code, Safari opened with the code filled
      in, and saved sign-ins for later launches.
- [x] More than one Microsoft account. The start screen lists them before the game starts, so you
      can play as one, add one, remove one, or play offline.
- [x] A chosen offline name for single player and offline-mode servers.
- [x] Touch controls: a floating move stick with sprint, action buttons, looking by dragging, and
      tap, scroll, drag and right-click in menus.
- [x] An Esc button.
- [x] Edit HUD can move, add, remove, resize and reset buttons. Buttons can press hotbar slots 1
      to 9, F1, F2, F3, the F3 combinations and `/`.
- [x] Edit HUD also moves and resizes the stick area and the Esc button, and keeps three layouts.
- [x] The iPhone keyboard opens when you tap a text box and closes with return or a tap anywhere
      else. Return sends chat. The controller's on-screen keyboard is off.
- [x] Chat has its own Tab, Paste and Enter buttons, and tapping chat's line lets you type again.
- [x] Copy and paste with other apps through `UIPasteboard`.
- [x] The game pauses and saves when the app leaves the screen.
- [x] Your files are in the Files app.
- [x] Servers behind an SRV redirect can be joined.
- [x] Simple Voice Chat, listening only: you hear the players who use it, through Horizon's
      voicechat engine with Opus linked into the app. The microphone is never used.
- [x] Improved transparency no longer crashes on iOS GPUs.
- [x] An About the Project page in Options.
- [x] Extra Settings (Ferrite) on the main menu: Safe Area as an option (backgrounds still
      fill the screen), button opacity, touch look speed, and keeping the screen on.
- [x] An AltStore and SideStore source, so new builds arrive as updates there.
- [x] The start screen says when a newer build is out.
- [x] Talking in Simple Voice Chat: the microphone, push to talk, mute, and the voice chat menu
      on Edit HUD buttons.
- [x] Native mods: the registry's mods that work on this port's Horizon are built into the app,
      off until each player turns them on, with Edit HUD buttons for their keys.

## Next

- Add/Improve performance and RAM limiting, Extra-Memory permission.
- Launcher screen in SwiftUI

## Later

- [ ] iPad: pointer and keyboard support, and Split View sizes. The maintainer doesn't own an
      iPad and can't test this. If you'd like to volunteer, DM `4rnz` on Discord.
- [ ] Game controllers, read through Apple's GameController framework. `gilrs` has no iOS
      backend, and the virtual pad already takes their input.
- [ ] A narrator, with text-to-speech through `AVSpeechSynthesizer`.

## Not planned

- The App Store or TestFlight. The app needs your own Minecraft files, and the game isn't ours to
  publish there.
- The desktop launcher's pages. Choosing other native mods, the mod browser and modpacks need a
  computer: an iPhone can't compile or load new code.
- Phone-friendly defaults. Horizon is already well optimized, so a lower render distance or
  graphics preset on first launch isn't needed.
