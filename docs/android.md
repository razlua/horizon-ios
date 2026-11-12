# Horizon for Android (unofficial)

The Android port lives on the `android-port` branch. It's the same game client as the iOS port,
built for Android as an `.apk`, with the same touch HUD, Edit HUD, start screen and Extra
Settings. It's experimental: it builds, but it has had very little testing on real devices.

## What you need

- Android 10 or later, on a 64-bit device: a phone or tablet (`arm64-v8a`), or Android on a PC,
  Chromebook or emulator (`x86_64`). The one `.apk` has both.
- A GPU with Vulkan. The game falls back to OpenGL ES where it has to.
- A Microsoft account that owns Minecraft: Java Edition.
- Wi-Fi for the first launch, which downloads about 520 MB.

## Installing

1. Download `Horizon.apk` and open it. Android asks you to allow installs from your browser or
   file manager the first time.
2. Open Horizon and tap **Open sign-in page**. Sign in with Microsoft in the browser, then come
   back to Horizon.
3. Wait while it gets the game files: your Minecraft 26.3 from Mojang, checked against Mojang's
   checksums. This happens once.

To update, install the new `.apk` over the old one. Your worlds and sign-ins stay, as long as
both builds are signed with the same key (release builds are).

## Where things are

The game folder is the app's folder on shared storage:
`Android/data/dev.playhorizon.unofficial.android/files`. A computer over USB shows it, and so do
file managers that can open `Android/data`.

| Path | What it is |
|---|---|
| `saves/` | Worlds |
| `options.txt`, `resourcepacks/` | Settings and packs |
| `touch-controls.json` | Your Edit HUD layouts |
| `ios-settings.txt` | Extra Settings (the file keeps its iOS name) |
| `home/logs/latest.log` | The log. Attach it to bug reports |
| `resources/` | The game files |

Sign-ins are in the app's private storage. Uninstalling the app deletes everything above.

## What's different from iOS

- The system's Back button or gesture is Esc.
- The system keyboard types through key events, so letters outside the basic Latin set (å, ä, ö
  and others) may not arrive yet.
- Safe Area in Extra Settings does nothing yet: the game draws over the whole screen, cutout
  included.
- There's no update notice yet.
- Voice chat asks for the microphone the first time you use a voice chat button.

## How it's built

`crates/horizon-android` is the app's native library (`libhorizon.so`): `android_main`, which
sets up the folders and starts the game. `NativeActivity` loads it; a small Java subclass
(`packaging/android/java`) only hides the system bars. winit's Android backend gives the window
and the input, wgpu draws with Vulkan, and cpal plays sound through AAudio.

The code the two phone ports share keeps its `ios_` names (`ios_boot.rs`, `ios_setup.rs`,
`ios_extras.rs`), so the branches merge easily. `crates/horizon-client/src/game/phone.rs` names
the system calls once for both, and `crates/horizon-threads/src/android_sys.rs` makes Android's
through JNI.

Android takes the window's surface away whenever the app leaves the screen. The renderer drops
it then (`Renderer::drop_surface`) and makes a new one when the app is back.

The [Android workflow](../.github/workflows/android.yml) builds on GitHub's Linux runners on
every push to `android-port`:

```sh
rustup target add aarch64-linux-android
# CC_aarch64_linux_android, AR_aarch64_linux_android and the linker point at the NDK's clang.
cargo build --profile fast --target aarch64-linux-android -p horizon-android
bash packaging/android/package.sh dist/Horizon.apk arm64-v8a=target/aarch64-linux-android/fast/libhorizon.so
```

`packaging/android/package.sh` packs the `.apk` with the SDK's own tools (aapt2, d8, zipalign,
apksigner), without Gradle. With no release key in the environment it signs with a throwaway
key, and such a build can't be installed over one signed with another key.
