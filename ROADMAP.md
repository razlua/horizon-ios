# Roadmap: Horizon for iOS

This roadmap covers the iOS port only. Upstream Horizon's own roadmap is kept in
[docs/upstream/ROADMAP.md](docs/upstream/ROADMAP.md). The game itself (world generation, mobs,
redstone, multiplayer and so on) comes from upstream.

## Done

- [x] The game client builds for `aarch64-apple-ios` on GitHub's macOS runners and is packaged
      as an unsigned `.ipa`.
- [x] Drawing with Metal, in a window that fills the screen, sideways only, at up to 120 Hz on
      ProMotion screens.
- [x] Your own game files added to the app on your computer, never uploaded. The app unpacks
      them on its first launch, beside the event loop, so iOS doesn't kill a slow start.
- [x] A start screen from the first frame, with a progress bar while the first launch unpacks.
- [x] Microsoft sign-in without a launcher: a device code, Safari opened with the code filled
      in, and saved sign-ins for later launches.
- [x] **Accounts.** More than one Microsoft account. The start screen lists them before the
      game starts: play as one, add one, remove one, or play offline.
- [x] Touch controls: a floating move stick with sprint, action buttons, look by dragging, and
      tap, scroll, drag and right-click in menus.
- [x] An Esc button.
- [x] Edit HUD: move, add, remove, resize and reset buttons. Buttons can press hotbar slots 1 to
      9, F1, F2, F3, the F3 combinations and `/`.
- [x] **Edit HUD extras.** Moving and resizing the stick area, moving the Esc button, and three
      saved layouts.
- [x] The iPhone keyboard opens on a tap on a text box and closes with return or a tap anywhere
      else. Return sends chat. The controller's on-screen keyboard is off.
- [x] Chat's own Tab, Paste and Enter buttons, and tapping chat's line to type again.
- [x] **Clipboard.** Copy and paste with other apps through `UIPasteboard`.
- [x] Pausing and saving when the app leaves the screen.
- [x] Your files in the Files app.
- [x] Servers behind an SRV redirect can be joined.
- [x] **Simple Voice Chat, listening.** Hearing the players who use Simple Voice Chat, through
      Horizon's voicechat engine, with Opus linked into the app. No microphone.
- [x] Improved transparency no longer crashes on iOS GPUs.

## Next

Nothing is queued right now: the items above were the last Next list. Suggestions are welcome.

## Later

- [ ] **No computer needed.** Prepare the game files on the device. This needs upstream's
      "preparing without Java" work (upstream issue #1750).
- [ ] **iPad.** Pointer and keyboard support and Split View sizes. The maintainer doesn't own
      an iPad and can't test this. If you'd like to volunteer, DM `4rnz` on Discord.
- [ ] **Updates in the app.** Show when a newer build exists.
- [ ] **Game controllers.** Read paired controllers through Apple's GameController framework.
      `gilrs` has no iOS backend, and the virtual pad already takes their input.
- [ ] **Narrator.** Text-to-speech through `AVSpeechSynthesizer`.
- [ ] **Voice chat, talking.** The microphone, push to talk, and the voice chat's menu on the
      touch controls.

## Not planned

- **Safe areas.** Keeping the HUD and screens clear of the Dynamic Island and the home
  indicator was tried and taken back out: the game draws edge to edge. Edit HUD moves the touch
  buttons where they suit you.

- **The App Store or TestFlight.** The app needs your own Minecraft files, and the game is not
  ours to publish there.
- **The desktop launcher's pages.** Native Rust mods, the mod browser and modpacks need a
  computer.
- **Phone-friendly defaults.** Horizon is already well optimized, so a lower render distance
  or graphics preset on first launch isn't needed.
