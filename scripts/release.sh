#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
VERSION=$(defaults read "$ROOT/resources/Info" CFBundleShortVersionString 2>/dev/null || /usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$ROOT/resources/Info.plist")
RELEASE_DIR="$ROOT/release"
ARCHIVE="$RELEASE_DIR/ZoomOSC-Lite-v$VERSION-macOS-arm64.zip"

sh "$ROOT/scripts/build-app.sh"
mkdir -p "$RELEASE_DIR"
rm -f "$ARCHIVE" "$ARCHIVE.sha256"
ditto -c -k --sequesterRsrc --keepParent "$ROOT/dist/ZoomOSC Lite.app" "$ARCHIVE"
(
  cd "$RELEASE_DIR"
  shasum -a 256 "$(basename "$ARCHIVE")" > "$(basename "$ARCHIVE").sha256"
)

echo "Release criada: $ARCHIVE"
