#!/usr/bin/env bash
# Packs the Android build of the game as an .apk, with the SDK's own tools (aapt2, d8,
# zipalign, apksigner): no Gradle. The .apk holds no Minecraft file; the app downloads the
# player's own after a Microsoft sign-in.
# Usage: packaging/android/package.sh <output .apk> <abi>=<libhorizon.so> [<abi>=<libhorizon.so>...]
# Environment: ANDROID_HOME; HORIZON_ANDROID_VERSION and HORIZON_ANDROID_BUILD (the version's
# name and number); HORIZON_ANDROID_KEYSTORE (a PKCS12 file, alias "horizon") and
# HORIZON_ANDROID_KEYSTORE_PASS to sign with the release key. Without them the .apk is signed
# with a key made on the spot: it installs, but not over a build signed with another key.
set -euo pipefail
out="$1"
shift
root="$(cd "$(dirname "$0")/../.." && pwd)"
sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:?ANDROID_HOME is not set}}"
tools="$(ls -d "$sdk"/build-tools/* | sort -V | tail -1)"
# The newest released platform: preview ones (android-37.2-beta3) are left out.
platform="$(ls -d "$sdk"/platforms/android-* | grep -E '/android-[0-9]+$' | sort -V | tail -1)/android.jar"
version="${HORIZON_ANDROID_VERSION:-0.1.0}"
build="${HORIZON_ANDROID_BUILD:-1}"
work="$(mktemp -d)"
echo "build-tools $tools, platform $platform, version $version ($build)"

# Resources: the theme and the icon.
cp -r "$root/packaging/android/res" "$work/res"
mkdir -p "$work/res/mipmap-xxxhdpi"
icon="$root/packaging/icons/hicolor/256x256/apps/horizon.png"
if command -v convert >/dev/null; then
  convert "$icon" -resize 192x192 "$work/res/mipmap-xxxhdpi/ic_launcher.png"
else
  cp "$icon" "$work/res/mipmap-xxxhdpi/ic_launcher.png"
fi
"$tools/aapt2" compile --dir "$work/res" -o "$work/res.zip"
"$tools/aapt2" link -o "$work/base.apk" -I "$platform" \
  --manifest "$root/packaging/android/AndroidManifest.xml" \
  --min-sdk-version 29 --target-sdk-version 35 \
  --version-code "$build" --version-name "$version" \
  "$work/res.zip"

# The activity.
mkdir -p "$work/classes" "$work/dex"
find "$root/packaging/android/java" -name "*.java" > "$work/sources.txt"
javac -source 11 -target 11 -Xlint:-options -classpath "$platform" -d "$work/classes" @"$work/sources.txt"
find "$work/classes" -name "*.class" > "$work/classes.txt"
"$tools/d8" --lib "$platform" --min-api 29 --output "$work/dex" @"$work/classes.txt"
(cd "$work/dex" && zip -q "$work/base.apk" classes.dex)

# The game, one library per ABI, and the notices.
for pair in "$@"; do
  abi="${pair%%=*}"
  library="${pair#*=}"
  mkdir -p "$work/lib/$abi"
  cp "$library" "$work/lib/$abi/libhorizon.so"
done
mkdir -p "$work/assets"
cp "$root/LICENSE" "$root/THIRD_PARTY_NOTICES.md" "$work/assets/"
(cd "$work" && zip -q -r "$work/base.apk" lib assets)

"$tools/zipalign" -f -p 4 "$work/base.apk" "$work/aligned.apk"
if [ -n "${HORIZON_ANDROID_KEYSTORE:-}" ]; then
  keystore="$HORIZON_ANDROID_KEYSTORE"
  export KEYSTORE_PASS="$HORIZON_ANDROID_KEYSTORE_PASS"
  echo "signing with the release key"
else
  keystore="$work/throwaway.p12"
  export KEYSTORE_PASS="throwaway"
  keytool -genkeypair -keystore "$keystore" -storetype PKCS12 -alias horizon -keyalg RSA \
    -keysize 2048 -validity 3650 -storepass "$KEYSTORE_PASS" \
    -dname "CN=Horizon for Android test build" >/dev/null 2>&1
  echo "signing with a throwaway key (no release key given)"
fi
mkdir -p "$(dirname "$out")"
"$tools/apksigner" sign --ks "$keystore" --ks-type PKCS12 --ks-key-alias horizon \
  --ks-pass env:KEYSTORE_PASS --v1-signing-enabled true --v2-signing-enabled true \
  --v3-signing-enabled true --out "$out" "$work/aligned.apk"
"$tools/apksigner" verify --verbose "$out" | head -6
ls -lh "$out"
unzip -l "$out" | tail -15
