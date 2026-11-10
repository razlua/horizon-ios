# Horizon on iOS (unofficial, experimental)

An unofficial build of the game for iPhone and iPad, sideloaded: there is no App Store or
TestFlight build, and upstream lists iOS as a long-term goal only (ROADMAP.md, "Android, iOS and
the browser"). It is the game alone. The launcher has no iOS counterpart: iOS lets an app neither
download and start another program nor compile native mods.

## What is in the build

`.github/workflows/ios.yml` builds `horizon-client` for `aarch64-apple-ios` on a macOS runner and
wraps it in `Horizon.app` (`packaging/ios/package.sh`, `packaging/ios/Info.plist`). The artifact
is `Horizon-unsigned.ipa`. It holds no Minecraft file.

## Your own resources

The game needs the resources the launcher prepared from your copy of Minecraft (the folder with
`horizon-resources.json`). Preparing runs Mojang's jar on a JVM, which iOS does not have, so it
is done on a computer:

```sh
python tools/ios/add-resources.py Horizon-unsigned.ipa <resources folder> Horizon.ipa
```

That adds them to the app as one `resources.zip`; the game unpacks it into the app's Documents
(`resources/`) at its first start, which takes a while. Without it, copy the folder to
"On My iPhone > Horizon > resources" with the Files app. The .ipa with your resources in it is
for your own device only.

## Installing

Sign and install `Horizon.ipa` with a sideloading tool (Sideloadly, AltStore, SideStore) and your
own Apple ID. A free Apple ID signs for 7 days at a time.

## Signing in

Every start opens a screen of its own (`game/ios_boot.rs`, `ios_screen.rs`). With no launcher
to hand the game a session, the app signs in itself (`game/ios_account.rs`): a Microsoft device
code with Cazza Launcher's Azure app (`967c7b5b-8b53-4975-b4e3-7c9a394ed3af`, personal accounts,
public client), then Xbox Live, XSTS and the Minecraft services. "Open sign-in page" opens Safari
with the code filled in; "Play offline" skips the sign-in. The first start signs in while the
resources unpack.

The Microsoft refresh token, the name and the uuid are kept in the app's
`Library/Application Support/horizon/account.json`, readable by the app only and not shown in
the Files app; later starts sign in with it without asking. A refused token is forgotten and the
code comes back. Deleting the app deletes it.

## Where things are

The game folder is the app's Documents, visible in the Files app: `saves/`, `options.txt`,
`resourcepacks/`, and `home/logs/latest.log` and `home/crash-reports/` for bug reports.
`ios-resources-error.txt` appears there when unpacking the resources failed.

## Touch controls

`crates/horizon-client/src/game/touch.rs`.

- In a screen: tap to click, swipe up or down to scroll, drag sideways (or hold, then drag) to
  drag, and tap a second finger while the first rests to right click.
- In the world: the left of the screen is a move stick that appears under the thumb (push past
  its edge to sprint), the buttons are Jump, Sneak, Use, Hit, pause, inventory, chat, drop,
  camera and the hotbar's previous and next, and a finger anywhere else turns the view.

A small Esc button at the top centre does what Escape does, in the world and in every screen.

**Edit HUD** (the title's button in Quit Game's place) moves, adds, removes and resizes the
buttons (`game/touch_editor.rs`). Besides the controller's actions there are keyboard ones: F1,
F2, F3, the F3 combinations (B, G, H, N, T, F4) and `/`, pressed through `horizon-ctl key`'s
path. The layout is kept in `touch-controls.json` in the game folder; Reset puts the defaults
back.

A controller paired with the phone is not read yet (gilrs has no iOS backend).

## What does not work

- Native Rust mods and the launcher's pages (instances, servers, mod browser).
- The system clipboard: text copied in the game pastes in the game only.
- Hardware controllers, the narrator.

## Checking it compiles without a Mac

`cargo check --target aarch64-apple-ios -p horizon-client --no-default-features --features
audio,system-allocator` type-checks everywhere once the C build scripts are given a stand-in
compiler (they need the iOS SDK). Linking needs macOS.
