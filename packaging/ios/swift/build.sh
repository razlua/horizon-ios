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
# The C compiler's runtime: Swift's `#available` checks (the launcher's Liquid Glass, iOS 26) call
# ___isPlatformVersionAtLeast from it, and the Rust link (cc -nodefaultlibs) doesn't add it.
clang_dir="$(cd "$(dirname "$(xcrun --find clang)")/../lib/clang" && pwd)"
clang_version="$(ls "$clang_dir" | sort -V | tail -1)"
case "$sdk_name" in
  iphoneos) clang_rt="$clang_dir/$clang_version/lib/darwin/libclang_rt.ios.a" ;;
  *) clang_rt="$clang_dir/$clang_version/lib/darwin/libclang_rt.iossim.a" ;;
esac
if [ ! -f "$clang_rt" ]; then
  echo "no compiler runtime at $clang_rt" >&2
  exit 1
fi
if [ -n "${GITHUB_ENV:-}" ]; then
  echo "HORIZON_IOS_SWIFT_LIB=$out/libHorizonUI.a" >> "$GITHUB_ENV"
  echo "HORIZON_IOS_SWIFT_RUNTIME=$runtime" >> "$GITHUB_ENV"
  echo "HORIZON_IOS_CLANG_RT=$clang_rt" >> "$GITHUB_ENV"
fi
echo "HORIZON_IOS_SWIFT_LIB=$out/libHorizonUI.a"
echo "HORIZON_IOS_SWIFT_RUNTIME=$runtime"
echo "HORIZON_IOS_CLANG_RT=$clang_rt"
