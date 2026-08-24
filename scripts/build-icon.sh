#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
SWIFT_CACHE="/tmp/zoomosc-lite-swift-module-cache"
GENERATOR="/tmp/zoomosc-lite-icon-generator"
SOURCE_PNG="/tmp/zoomosc-lite-icon-1024.png"
ICONSET="/tmp/ZoomOSCLite.iconset"

mkdir -p "$SWIFT_CACHE"
rm -rf "$ICONSET"
mkdir -p "$ICONSET"

swiftc -module-cache-path "$SWIFT_CACHE" "$ROOT/macos/generate-icon.swift" -o "$GENERATOR"
"$GENERATOR" "$SOURCE_PNG"

sips -z 16 16 "$SOURCE_PNG" --out "$ICONSET/icon_16x16.png" >/dev/null
sips -z 32 32 "$SOURCE_PNG" --out "$ICONSET/icon_16x16@2x.png" >/dev/null
sips -z 32 32 "$SOURCE_PNG" --out "$ICONSET/icon_32x32.png" >/dev/null
sips -z 64 64 "$SOURCE_PNG" --out "$ICONSET/icon_32x32@2x.png" >/dev/null
sips -z 128 128 "$SOURCE_PNG" --out "$ICONSET/icon_128x128.png" >/dev/null
sips -z 256 256 "$SOURCE_PNG" --out "$ICONSET/icon_128x128@2x.png" >/dev/null
sips -z 256 256 "$SOURCE_PNG" --out "$ICONSET/icon_256x256.png" >/dev/null
sips -z 512 512 "$SOURCE_PNG" --out "$ICONSET/icon_256x256@2x.png" >/dev/null
sips -z 512 512 "$SOURCE_PNG" --out "$ICONSET/icon_512x512.png" >/dev/null
cp "$SOURCE_PNG" "$ICONSET/icon_512x512@2x.png"

python3 "$ROOT/scripts/pngs-to-icns.py" "$ICONSET" "$ROOT/resources/ZoomOSCLite.icns"
