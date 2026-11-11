#!/usr/bin/env bash
# Wraps the iOS build of the game in Horizon.app and zips it as an unsigned .ipa.
# The .ipa holds no Minecraft file: the prepared resources are added on the player's own
# computer (tools/ios/add-resources.py) or copied into the app's Documents with the Files app.
# Usage: packaging/ios/package.sh <path to the horizon-client binary> <output .ipa>
set -euo pipefail
binary="$1"
out="$2"
root="$(cd "$(dirname "$0")/../.." && pwd)"
work="$(mktemp -d)"
app="$work/Payload/Horizon.app"
mkdir -p "$app"
cp "$binary" "$app/horizon-client"
chmod 755 "$app/horizon-client"
cp "$root/packaging/ios/Info.plist" "$app/Info.plist"
# Each build its own version, so AltStore and SideStore see a new one as an update.
if [ -n "${HORIZON_IOS_VERSION:-}" ]; then
  plutil -replace CFBundleShortVersionString -string "$HORIZON_IOS_VERSION" "$app/Info.plist"
fi
if [ -n "${HORIZON_IOS_BUILD:-}" ]; then
  plutil -replace CFBundleVersion -string "$HORIZON_IOS_BUILD" "$app/Info.plist"
fi
printf 'APPL????' > "$app/PkgInfo"
cp "$root/LICENSE" "$root/THIRD_PARTY_NOTICES.md" "$app/"
icon="$root/packaging/icons/hicolor/512x512/apps/horizon.png"
sips -z 120 120 "$icon" --out "$app/AppIcon60x60@2x.png" >/dev/null
sips -z 180 180 "$icon" --out "$app/AppIcon60x60@3x.png" >/dev/null
sips -z 152 152 "$icon" --out "$app/AppIcon76x76@2x~ipad.png" >/dev/null
plutil -lint "$app/Info.plist"
# An ad hoc signature: sideloading tools replace it with the player's own.
codesign --force --sign - --timestamp=none "$app" || true
mkdir -p "$(dirname "$out")"
out="$(cd "$(dirname "$out")" && pwd)/$(basename "$out")"
rm -f "$out"
(cd "$work" && zip -qry "$out" Payload)
ls -lh "$out"
file "$app/horizon-client"
otool -l "$app/horizon-client" | grep -A4 -E 'LC_BUILD_VERSION|LC_MAIN' || true
