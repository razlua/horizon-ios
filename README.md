# Horizon for iOS (unofficial)

An unofficial, experimental port of [Horizon](https://github.com/horizon-rust/horizon), the Rust
rewrite of Minecraft: Java Edition 26.3, to iPhone and iPad. You sideload it yourself.

> **Not official.** This port is not made, approved or supported by the Horizon project, Mojang
> or Microsoft. It contains **no Minecraft code, assets or data**. To play, you need your own copy
> of Minecraft: Java Edition, which the Horizon launcher on a computer turns into the files the
> game reads. Please report problems with this port here, not to upstream Horizon.

- **Game:** Horizon's own game client, built for `aarch64-apple-ios` and drawn with Metal.
- **Base:** upstream Horizon at commit `6475f62`. Upstream's README and roadmap are kept in
  [docs/upstream/](docs/upstream/).
- **Status:** experimental. It starts, signs in, and plays singleplayer worlds and servers on a
  recent iPhone. Expect rough edges. See [ROADMAP.md](ROADMAP.md) and the known issues below.

## What works

- **Start screen.** It shows from the very first frame. The first launch unpacks your game files
  in about a minute, with a progress bar, and you can sign in while it does.
- **Microsoft sign-in, without a launcher.** Tap **Open sign-in page** and Safari opens with your
  code already filled in. **Play offline** skips it.
- **Several accounts.** The start screen lists your saved Microsoft accounts: tap one to play as
  it, or add or remove accounts.
- **Touch controls.** In the world, the left side of the screen is a floating move stick (push
  past its edge to sprint), the buttons on the right act, and a finger anywhere else turns the
  view. In menus you tap to click, swipe to scroll, and tap with a second finger to right-click.
- **Esc button.** A small Esc button sits at the top centre. It closes the inventory and other
  screens, and in the world it opens the pause menu.
- **Edit HUD.** On iOS this button takes Quit Game's place on the main menu. You can drag
  buttons, add and remove them and resize them, move and resize the move stick's area, and move
  the Esc button. Three layouts can be kept, and Reset brings back the defaults. Buttons can press keyboard keys too: hotbar slots 1 to 9, F1, F2, F3,
  F3+B / G / H / N / T / F4, and `/`.
- **Keyboard and chat.** The iPhone keyboard opens when you tap a text box, including chat's
  line. Return sends a chat message or confirms, then closes the keyboard, and so does tapping
  anywhere else. Over chat, **Tab** completes commands and names, **Paste** pastes, and
  **Enter** sends.
- **Clipboard.** Copy and paste work with other apps. iOS asks before the game reads what you
  copied.
- **120 Hz** on ProMotion screens.
- **Simple Voice Chat, listening.** On servers with the Simple Voice Chat plugin or mod, you hear
  the players who use it, from where they stand. The iPhone doesn't send your voice: it never
  opens the microphone. The client is unofficial (Horizon's voicechat native mod's engine) and
  not affiliated with Simple Voice Chat.
- **Your files in the Files app.** Saves, options, resource packs and logs are under
  *On My iPhone → Horizon*.
- **Pausing in the background.** The game pauses and saves when you leave the app.

## Installing

What you need:

- **An iPhone or iPad** with iOS 16 or later. Developer Mode must be on (*Settings → Privacy &
  Security → Developer Mode*).
- **A computer** with the [Horizon launcher](https://playhorizon.dev) that has prepared a game
  version, or the prepared resources folder copied from one.
- **A sideloading tool** such as [Sideloadly](https://sideloadly.io) or
  [AltStore](https://altstore.io), and your own Apple ID.

Steps:

1. **Get the app.** Download `Horizon-ios-unsigned` from the latest successful
   [iOS workflow run](../../actions/workflows/ios.yml). It holds `Horizon-unsigned.ipa`, about
   30 MB.
2. **Add your game files** on the computer. The prepared folder is the one holding
   `horizon-resources.json`, usually
   `<launcher data>/versions/<version>/.local/horizon-client/26.3`. Run:

   ```sh
   python tools/ios/add-resources.py Horizon-unsigned.ipa "<prepared folder>" Horizon.ipa
   ```

   The resulting `Horizon.ipa` (about 450 MB) contains your own copy of the game's files. It is
   for your device only, so **do not share it**.
3. **Sideload `Horizon.ipa`** with your tool.
4. **Trust your profile** on the phone: *Settings → General → VPN & Device Management → your
   Apple ID → Trust*. The phone needs internet for this, and a VPN or DNS filter can block it.
5. **Open Horizon.** The first launch unpacks the game files, then shows the sign-in.

With a free Apple ID, the app has to be signed again every 7 days, and you can have at most 3
sideloaded apps at once.

## Controls

| Where | Do | What happens |
|---|---|---|
| World | Thumb on the left side | Move. Push past the edge to sprint |
| World | Drag anywhere else | Look around |
| World | Jump, Sneak, Use, Hit | Jump, sneak, use or place, attack or mine. Drag a button to aim while holding it |
| World | Inv, Chat, Drop, Cam, `<` `>` | Inventory, chat, drop item, camera, hotbar left and right |
| Anywhere | Esc (top centre) | Same as the Escape key |
| Chat | Tab, Paste, Enter (top right) | Complete, paste, send |
| Chat | Tap the line at the bottom | Type again |
| Menus | Tap | Click |
| Menus | Swipe up or down | Scroll |
| Menus | Drag sideways, or hold then drag | Drag sliders and items |
| Menus | Second finger tap | Right-click |
| Main menu | Edit HUD | Change the touch buttons |

## Where things are

On the phone, under *Files → On My iPhone → Horizon*:

| Path | What it is |
|---|---|
| `saves/` | Worlds |
| `options.txt`, `resourcepacks/` | Settings and packs |
| `touch-controls.json` | Your Edit HUD layouts. Delete it to go back to the defaults |
| `home/logs/latest.log` | The log. Attach it to bug reports |
| `home/crash-reports/` | Crash reports |
| `ios-*-error.txt` | Written when unpacking or starting failed |
| `resources/` | The unpacked game files |

Your Microsoft accounts' sign-ins are kept in the app's private storage, not in Files. Deleting
the app deletes them.

## Known issues

- **Controllers.** A paired game controller is not read yet.
- **Narrator.** There is no text-to-speech.
- **Voice chat, talking.** You can hear Simple Voice Chat but not speak, and its menu (V on a
  keyboard) isn't on the touch controls.
- **Screen edges.** The game draws edge to edge, so part of it can sit under the Dynamic Island,
  the notch or the home indicator. Edit HUD can move the touch buttons away from them.
- **Missing launcher features.** Native Rust mods, the mod browser, modpacks and server hosting
  need the desktop launcher and don't exist here.
- **Fresh install.** Preparing the game files needs Java, which iOS doesn't have, so a computer is
  needed once.

More in [ROADMAP.md](ROADMAP.md). Changes are listed in [CHANGELOG.md](CHANGELOG.md).

## Building

You need macOS to link an iOS app. The [iOS workflow](.github/workflows/ios.yml) builds on
GitHub's macOS runners on every push to `ios-port`:

```sh
rustup target add aarch64-apple-ios
cargo build --profile fast --target aarch64-apple-ios -p horizon-client --bin horizon-client \
  --no-default-features --features audio,system-allocator
bash packaging/ios/package.sh target/aarch64-apple-ios/fast/horizon-client dist/Horizon-unsigned.ipa
```

On any other system the same command with `cargo check` type-checks the port, as long as the C
build scripts get a stand-in compiler (they want the iOS SDK). [docs/ios.md](docs/ios.md)
describes how the port fits together.

### Where the iOS code is

| File | What it does |
|---|---|
| `crates/horizon-client/src/game/ios_boot.rs` | The start screen; runs the unpacking and the sign-in beside the event loop |
| `crates/horizon-client/src/game/ios_account.rs` | Microsoft → Xbox Live → XSTS → Minecraft services sign-in, and the saved accounts |
| `crates/horizon-client/src/game/touch.rs` | Touch controls, the Esc button, key buttons |
| `crates/horizon-client/src/game/touch_editor.rs` | Edit HUD |
| `crates/horizon-client/src/ios_screen.rs` | The start screen's drawing |
| `crates/horizon-threads/src/ios_url.rs`, `ios_pasteboard.rs`, `ios_display.rs` | Opening Safari, the clipboard, the frame rate |
| `packaging/ios/`, `tools/ios/add-resources.py` | The app bundle and adding your game files |

## Credits and license

- **Horizon:** Ludovic Marie and the Horizon contributors. Every part of the game is theirs.
  This fork adds the iOS pieces.
- **Microsoft sign-in:** uses Cazza Launcher's Azure application.
- **License:** MIT, the same as upstream ([LICENSE](LICENSE)). Third-party notices are in
  [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

Minecraft is a trademark of Mojang Synergies AB. This project is not affiliated with Mojang or
Microsoft.
