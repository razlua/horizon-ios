# Horizon for iOS (unofficial)

An unofficial, experimental port of [Horizon](https://github.com/horizon-rust/horizon), the Rust
rewrite of Minecraft: Java Edition 26.3, to iPhone and iPad. You sideload it yourself.

> This port isn't made, approved or supported by the Horizon project, Mojang or Microsoft. It
> contains no Minecraft code, assets or data: the app downloads your own copy of Minecraft from
> Mojang after you sign in with an account that owns Java Edition. Please report problems with
> this port here, not to upstream Horizon.

It's Horizon's own game client, built for `aarch64-apple-ios` and drawn with Metal, on top of
upstream Horizon at commit `6475f62`. Upstream's README and roadmap are kept in
[docs/upstream/](docs/upstream/). It starts, signs in, and plays single player worlds and servers
on a recent iPhone, but expect rough edges. See [ROADMAP.md](ROADMAP.md) and the known issues
below.

## What works

- The start screen shows from the first frame. On the first launch it downloads and sets up the
  game files with a progress bar, after you sign in.
- Microsoft sign-in works without a launcher: tap Open sign-in page and Safari opens with your
  code already filled in.
- The start screen lists your saved Microsoft accounts. Tap one to play as it, or add or remove
  accounts.
- Play offline lets you pick the name you use in single player and on offline-mode ("cracked")
  servers.
- In the world, the left side of the screen is a floating move stick (push past its edge to
  sprint), the buttons on the right do the actions, and a finger anywhere else turns the view. In
  menus you tap to click, swipe to scroll, and tap with a second finger to right-click.
- A small Esc button at the top centre closes the inventory and other screens, and opens the
  pause menu in the world.
- Edit HUD takes Quit Game's place on the main menu. You can drag, add, remove and resize buttons,
  move and resize the move stick's area, and move and resize the Esc button. It keeps three
  layouts, and Reset brings back the defaults. Buttons can also press keyboard keys: hotbar slots
  1 to 9, F1, F2, F3, F3+B / G / H / N / T / F4, and `/`.
- The iPhone keyboard opens when you tap a text box, including chat's line. Return sends a chat
  message or confirms, then closes the keyboard, and tapping anywhere else closes it too. Over
  chat, Tab completes commands and names, Paste pastes, and Enter sends.
- Copy and paste work with other apps. iOS asks before the game reads what you copied.
- ProMotion screens run at 120 Hz.
- On servers with the Simple Voice Chat plugin or mod, you hear the players who use it, coming
  from where they stand. You can't be heard, because the app never opens the microphone. The
  client is unofficial (the engine of Horizon's voicechat native mod) and not affiliated with
  Simple Voice Chat.
- Saves, options, resource packs and logs are in the Files app, under *On My iPhone → Horizon*.
- The game pauses and saves when you leave the app.

## Installing

You need an iPhone or iPad on iOS 16 or later with Developer Mode on (*Settings → Privacy &
Security → Developer Mode*), a Microsoft account that owns Minecraft: Java Edition, and a
sideloading tool such as [Sideloadly](https://sideloadly.io) or [AltStore](https://altstore.io)
with your own Apple ID.

1. Download `Horizon-unsigned.ipa` (about 30 MB) from the latest
   [release](../../releases/latest).
2. Sideload it with your tool.
3. Trust your profile on the phone: *Settings → General → VPN & Device Management → your Apple ID
   → Trust*. The phone needs internet for this, and a VPN or DNS filter can block it.
4. Open Horizon and sign in with Microsoft. The first time, the app downloads your Minecraft 26.3
   from Mojang (about 520 MB) and Horizon's tables, then sets them up. Stay on Wi-Fi with Horizon
   open. After that it starts straight away, offline too.

With a free Apple ID the app has to be signed again every 7 days, and you can have at most 3
sideloaded apps at once.

### How the game files get there

Horizon's preparation runs a few of Mojang's Java classes to work out data tables (blocks,
entities, collision, world generation), and a phone can't run Java. Those tables are the same for
everyone on a version, so they're worked out once and published in this repository's
`tables-<revision>` release. That release has none of Minecraft's files in it, so no textures,
sounds, models or languages. The app downloads those from Mojang once you've signed in with an
account that owns the game. It checks every file against Mojang's checksum and the tables against
the SHA-256 built into the app. The folder it ends up with is identical, file for file, to what a
computer's preparation makes.

You can also add the game files on a computer instead. Run
`python tools/ios/add-resources.py Horizon-unsigned.ipa "<prepared folder>" Horizon.ipa` on the
folder the [Horizon launcher](https://playhorizon.dev) prepared, and sideload `Horizon.ipa`.
That app has your game files inside, so don't share it.

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
| `resources/` | The game files |

Your Microsoft sign-ins are kept in the app's private storage, not in Files, and deleting the app
deletes them.

## About

*Options → About the Project* says who makes this port and where to report bugs. That's @4rnz on
Discord (razlua on GitHub). Send bugs there, with `home/logs/latest.log` if you can.

## Known issues

- Paired game controllers aren't read yet.
- There's no narrator (text-to-speech).
- You can hear Simple Voice Chat but not talk, and its menu (V on a keyboard) isn't on the touch
  controls.
- The game draws edge to edge, so part of it can end up under the Dynamic Island, the notch or
  the home indicator. Edit HUD can move the touch buttons out of the way.
- Native Rust mods, the mod browser, modpacks and server hosting need the desktop launcher and
  aren't in this port.

More in [ROADMAP.md](ROADMAP.md). Changes are listed in [CHANGELOG.md](CHANGELOG.md).

## Building

Linking an iOS app needs macOS. The [iOS workflow](.github/workflows/ios.yml) builds on GitHub's
macOS runners on every push to `ios-port`:

```sh
rustup target add aarch64-apple-ios
cargo build --profile fast --target aarch64-apple-ios -p horizon-client --bin horizon-client \
  --no-default-features --features audio,system-allocator
bash packaging/ios/package.sh target/aarch64-apple-ios/fast/horizon-client dist/Horizon-unsigned.ipa
```

On other systems, the same command with `cargo check` type-checks the port, as long as the C
build scripts get a stand-in compiler (they want the iOS SDK). [docs/ios.md](docs/ios.md) explains
how the port fits together.

### Where the iOS code is

| File | What it does |
|---|---|
| `crates/horizon-client/src/game/ios_boot.rs` | The start screen; runs the setup and the sign-in beside the event loop |
| `crates/horizon-client/src/game/ios_setup.rs` | Downloads the game files from Mojang and the tables, and puts them together |
| `crates/horizon-client/src/game/ios_account.rs` | Microsoft → Xbox Live → XSTS → Minecraft services sign-in, and the saved accounts |
| `crates/horizon-client/src/game/touch.rs` | Touch controls, the Esc button, key buttons |
| `crates/horizon-client/src/game/touch_editor.rs` | Edit HUD |
| `crates/horizon-client/src/ios_screen.rs` | The start screen's drawing |
| `crates/horizon-threads/src/ios_url.rs`, `ios_pasteboard.rs`, `ios_display.rs` | Opening Safari, the clipboard, the frame rate |
| `packaging/ios/`, `tools/ios/` | The app bundle, adding game files on a computer, and building the tables |

## Credits and license

Horizon is by Ludovic Marie and the Horizon contributors, and the whole game is theirs; this fork
adds the iOS parts. Microsoft sign-in uses Cazza Launcher's Azure application. The license is
MIT, the same as upstream ([LICENSE](LICENSE)), and third-party notices are in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

Minecraft is a trademark of Mojang Synergies AB. This project isn't affiliated with Mojang or
Microsoft.
