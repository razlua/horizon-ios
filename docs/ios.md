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

A controller paired with the phone is not read yet (gilrs has no iOS backend).

## What does not work

- Native Rust mods and the launcher's pages (instances, servers, mod browser).
- Signing in: the launcher handed the game its Microsoft session, so the iOS build plays
  offline (singleplayer, LAN, offline-mode servers).
- The system clipboard: text copied in the game pastes in the game only.
- Hardware controllers, the narrator.

## Checking it compiles without a Mac

`cargo check --target aarch64-apple-ios -p horizon-client --no-default-features --features
audio,system-allocator` type-checks everywhere once the C build scripts are given a stand-in
compiler (they need the iOS SDK). Linking needs macOS.
