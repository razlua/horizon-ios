# Roadmap: Horizon for iOS

This roadmap covers the iOS port only. Upstream Horizon's own roadmap is kept in
[docs/upstream/ROADMAP.md](docs/upstream/ROADMAP.md). The game itself (world generation, mobs,
redstone, multiplayer and so on) comes from upstream.

## Done

- [x] The game client builds for `aarch64-apple-ios` on GitHub's macOS runners and is packaged
      as an unsigned `.ipa`.
- [x] Drawing with Metal, in a window that fills the screen, sideways only, at up to 120 Hz.
- [x] Your own game files added to the app on your computer, never uploaded. The app unpacks
      them on its first launch, beside the event loop, so iOS doesn't kill a slow start.
- [x] A start screen from the first frame.
- [x] Microsoft sign-in without a launcher: a device code, Safari opened with the code filled
      in, and a saved sign-in for later launches.
- [x] Touch controls: a floating move stick with sprint, action buttons, look by dragging, and
      tap, scroll, drag and right-click in menus.
- [x] An Esc button.
- [x] Edit HUD: move, add, remove, resize and reset buttons. Buttons can press F1, F2, F3, the
      F3 combinations and `/`.
- [x] The iPhone keyboard opens on a tap on a text box and closes with return or a tap anywhere
      else.
- [x] Pausing and saving when the app leaves the screen.
- [x] Your files in the Files app.
- [x] Improved transparency no longer crashes on iOS GPUs.

## Next

- [ ] **Safe areas.** Keep the HUD, the buttons and screens clear of the Dynamic Island, the
      notch and the home indicator.
- [ ] **Game controllers.** Read paired controllers through Apple's GameController framework.
      `gilrs` has no iOS backend, and the virtual pad already takes their input.
- [ ] **Clipboard.** Copy and paste with other apps through `UIPasteboard`.
- [ ] **Phone-friendly defaults.** A lower render distance and graphics preset on the first
      launch, and a battery-saving frame cap.
- [ ] **Unpacking progress.** A progress bar on the start screen while the first launch unpacks.
- [ ] **Haptics.** A light tap on button presses and when hit, where the game already rumbles a
      controller.
- [ ] **Edit HUD extras.** Button opacity, moving the stick area and the Esc button, and
      several saved layouts.
- [ ] **Narrator.** Text-to-speech through `AVSpeechSynthesizer`.

## Later

- [ ] **No computer needed.** Prepare the game files on the device. This needs upstream's
      "preparing without Java" work (upstream issue #1750).
- [ ] **iPad.** Pointer and keyboard support and Split View sizes.
- [ ] **Updates in the app.** Show when a newer build exists.
- [ ] **Accounts.** More than one Microsoft account, and switching between them.

## Not planned

- **The App Store or TestFlight.** The app needs your own Minecraft files, and the game is not
  ours to publish there.
- **The desktop launcher's pages.** Native Rust mods, the mod browser, modpacks and server
  hosting need a computer.
