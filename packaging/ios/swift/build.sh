#!/usr/bin/env bash
# Compiles the SwiftUI launcher (Launcher.swift and the other .swift files here) into a static
# library for the Rust build to link (crates/horizon-threads/build.rs), and tells the following
# steps where it is (HORIZON_IOS_SWIFT_LIB, HORIZON_IOS_SWIFT_RUNTIME in $GITHUB_ENV).
# Usage: packaging/ios/swift/build.sh <sdk: iphoneos | iphonesimulator> <swiftc target>
# e.g.   packaging/ios/swift/build.sh iphoneos arm64-apple-ios16.0
set -euo pipefail
sdk_name="$1"
target="$2"
here="$(cd "$(dirname "$0")" && pwd)"
out="${RUNNER_TEMP:-/tmp}/swift-$sdk_name"
mkdir -p "$out"
sdk="$(xcrun --sdk "$sdk_name" --show-sdk-path)"
xcrun --sdk "$sdk_name" swiftc -O -wmo -parse-as-library -module-name HorizonUI \
  -target "$target" -sdk "$sdk" -emit-library -static \
  "$here"/*.swift -o "$out/libHorizonUI.a"
ls -lh "$out/libHorizonUI.a"
# The Swift libraries the launcher links to: the SDK's (the system's own, on the phone) and the
# toolchain's back-deployment ones (swiftCompatibility56 and the like).
toolchain="$(cd "$(dirname "$(xcrun --find swiftc)")/../lib/swift/$sdk_name" && pwd)"
runtime="$sdk/usr/lib/swift:$toolchain"
if [ -n "${GITHUB_ENV:-}" ]; then
  echo "HORIZON_IOS_SWIFT_LIB=$out/libHorizonUI.a" >> "$GITHUB_ENV"
  echo "HORIZON_IOS_SWIFT_RUNTIME=$runtime" >> "$GITHUB_ENV"
fi
echo "HORIZON_IOS_SWIFT_LIB=$out/libHorizonUI.a"
echo "HORIZON_IOS_SWIFT_RUNTIME=$runtime"
