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
  in about a minute, and you can sign in while it does.
- **Microsoft sign-in, without a launcher.** Tap **Open sign-in page** and Safari opens with your
  code already filled in. After that the app signs you in by itself. **Play offline** skips it.
- **Touch controls.** In the world, the left side of the screen is a floating move stick (push
  past its edge to sprint), the buttons on the right act, and a finger anywhere else turns the
  view. In menus you tap to click, swipe to scroll, and tap with a second finger to right-click.
- **Esc button.** A small Esc button sits at the top centre. It closes the inventory and other
  screens, and in the world it opens the pause menu.
- **Edit HUD.** On iOS this button takes Quit Game's place on the main menu. You can drag
  buttons, add and remove them, resize them, or reset to the defaults. Buttons can press keyboard
  keys too: F1, F2, F3, F3+B / G / H / N / T / F4, and `/`.
- **Keyboard.** The iPhone keyboard opens when you tap a text box. Return confirms and closes it,
  and so does tapping anywhere else.
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
| `touch-controls.json` | Your Edit HUD layout. Delete it to go back to the defaults |
| `home/logs/latest.log` | The log. Attach it to bug reports |
| `home/crash-reports/` | Crash reports |
| `ios-*-error.txt` | Written when unpacking or starting failed |
| `resources/` | The unpacked game files |

The Microsoft sign-in is kept in the app's private storage, not in Files. Deleting the app
deletes it.

## Known issues

- **Screen edges.** The game draws edge to edge, so part of it can sit under the Dynamic Island
  or notch.
- **Controllers.** A paired game controller is not read yet.
- **Clipboard.** Copy and paste only work inside the game, not with other apps.
- **Narrator.** There is no text-to-speech.
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
| `crates/horizon-client/src/game/ios_account.rs` | Microsoft → Xbox Live → XSTS → Minecraft services sign-in |
| `crates/horizon-client/src/game/touch.rs` | Touch controls, the Esc button, key buttons |
| `crates/horizon-client/src/game/touch_editor.rs` | Edit HUD |
| `crates/horizon-client/src/ios_screen.rs` | The start screen's drawing |
| `crates/horizon-threads/src/ios_url.rs` | Opening Safari |
| `packaging/ios/`, `tools/ios/add-resources.py` | The app bundle and adding your game files |

## Credits and license

- **Horizon:** Ludovic Marie and the Horizon contributors. Every part of the game is theirs.
  This fork adds the iOS pieces.
- **Microsoft sign-in:** uses Cazza Launcher's Azure application.
- **License:** MIT, the same as upstream ([LICENSE](LICENSE)). Third-party notices are in
  [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

Minecraft is a trademark of Mojang Synergies AB. This project is not affiliated with Mojang or
Microsoft.
