#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
APP="$ROOT/dist/ZoomOSC Lite.app"
SWIFT_CACHE="/tmp/zoomosc-lite-swift-module-cache"

cargo build --release
sh "$ROOT/scripts/build-icon.sh"
if [ -d "$APP" ]; then
  PREVIOUS="/tmp/ZoomOSC-Lite-previous-$$.app"
  mv "$APP" "$PREVIOUS"
  trap 'rm -rf "$PREVIOUS"' EXIT HUP INT TERM
fi
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
mkdir -p "$SWIFT_CACHE"
swiftc -parse-as-library \
  -module-cache-path "$SWIFT_CACHE" \
  -framework SwiftUI \
  -framework AppKit \
  "$ROOT/macos/ZoomOSCLiteApp.swift" \
  "$ROOT/target/release/libzoomosc_lite.a" \
  -o "$APP/Contents/MacOS/ZoomOSCLite"
cp "$ROOT/resources/Info.plist" "$APP/Contents/Info.plist"
cp "$ROOT/resources/ZoomOSCLite.icns" "$APP/Contents/Resources/ZoomOSCLite.icns"
codesign --force --deep --sign - "$APP"

codesign --verify --deep --strict "$APP"

echo "Criada: $APP"
