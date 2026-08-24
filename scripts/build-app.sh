#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
APP="$ROOT/dist/ZoomOSC Lite.app"
SWIFT_CACHE="/tmp/zoomosc-lite-swift-module-cache"
ARM_EXECUTABLE="/tmp/zoomosc-lite-arm64"
INTEL_EXECUTABLE="/tmp/zoomosc-lite-x86_64"

cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
sh "$ROOT/scripts/build-icon.sh"
if [ -d "$APP" ]; then
  PREVIOUS="/tmp/ZoomOSC-Lite-previous-$$.app"
  mv "$APP" "$PREVIOUS"
  trap 'rm -rf "$PREVIOUS"' EXIT HUP INT TERM
fi
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
mkdir -p "$SWIFT_CACHE/arm64" "$SWIFT_CACHE/x86_64"
swiftc -parse-as-library \
  -target arm64-apple-macosx13.0 \
  -module-cache-path "$SWIFT_CACHE/arm64" \
  -framework SwiftUI \
  -framework AppKit \
  -framework ServiceManagement \
  "$ROOT/macos/ZoomOSCLiteApp.swift" \
  "$ROOT/target/aarch64-apple-darwin/release/libzoomosc_lite.a" \
  -o "$ARM_EXECUTABLE"
swiftc -parse-as-library \
  -target x86_64-apple-macosx13.0 \
  -module-cache-path "$SWIFT_CACHE/x86_64" \
  -framework SwiftUI \
  -framework AppKit \
  -framework ServiceManagement \
  "$ROOT/macos/ZoomOSCLiteApp.swift" \
  "$ROOT/target/x86_64-apple-darwin/release/libzoomosc_lite.a" \
  -o "$INTEL_EXECUTABLE"
lipo -create "$ARM_EXECUTABLE" "$INTEL_EXECUTABLE" \
  -output "$APP/Contents/MacOS/ZoomOSCLite"
cp "$ROOT/resources/Info.plist" "$APP/Contents/Info.plist"
cp "$ROOT/resources/ZoomOSCLite.icns" "$APP/Contents/Resources/ZoomOSCLite.icns"
codesign --force --deep --sign - "$APP"

codesign --verify --deep --strict "$APP"
lipo "$APP/Contents/MacOS/ZoomOSCLite" -verify_arch arm64 x86_64

echo "Criada: $APP"
