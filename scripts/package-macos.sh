#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_NAME="LingXi T3 Native"
BUNDLE_ID="com.lingxifox.lingxit3.native"
BIN_NAME="lingxi-t3-native"
VERSION="${1:-0.1.0}"
ARCH="$(uname -m)"
DIST_DIR="$ROOT_DIR/dist"
APP_BUNDLE="$DIST_DIR/$APP_NAME.app"
DMG_PATH="$DIST_DIR/LingXiT3-Native-$VERSION-macos-$ARCH.dmg"

cd "$ROOT_DIR"
cargo build --release --bin "$BIN_NAME"

rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS" "$APP_BUNDLE/Contents/Resources"
cp "$ROOT_DIR/target/release/$BIN_NAME" "$APP_BUNDLE/Contents/MacOS/$BIN_NAME"
chmod +x "$APP_BUNDLE/Contents/MacOS/$BIN_NAME"

# Build .icns from assets/icons/icon-512.png using macOS sips + iconutil
ICON_SRC="$ROOT_DIR/assets/icons/icon-512.png"
if [[ -f "$ICON_SRC" ]] && command -v iconutil >/dev/null 2>&1; then
  ICONSET_DIR="$DIST_DIR/LingXiT3Native.iconset"
  rm -rf "$ICONSET_DIR"
  mkdir -p "$ICONSET_DIR"
  for sz in 16 32 128 256 512; do
    sips -z "$sz" "$sz" "$ICON_SRC" --out "$ICONSET_DIR/icon_${sz}x${sz}.png" >/dev/null
  done
  iconutil -c icns "$ICONSET_DIR" -o "$APP_BUNDLE/Contents/Resources/AppIcon.icns"
  rm -rf "$ICONSET_DIR"
fi

cat > "$APP_BUNDLE/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>$APP_NAME</string>
  <key>CFBundleDisplayName</key>
  <string>$APP_NAME</string>
  <key>CFBundleIdentifier</key>
  <string>$BUNDLE_ID</string>
  <key>CFBundleVersion</key>
  <string>$VERSION</string>
  <key>CFBundleShortVersionString</key>
  <string>$VERSION</string>
  <key>CFBundleExecutable</key>
  <string>$BIN_NAME</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>LSMinimumSystemVersion</key>
  <string>13.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>CFBundleURLTypes</key>
  <array>
    <dict>
      <key>CFBundleURLName</key>
      <string>$BUNDLE_ID</string>
      <key>CFBundleURLSchemes</key>
      <array>
        <string>lingxit3-native</string>
      </array>
    </dict>
  </array>
</dict>
</plist>
EOF

if [[ -n "${APPLE_DEVELOPER_ID:-}" ]]; then
  echo "Signing $APP_BUNDLE with Developer ID: $APPLE_DEVELOPER_ID"
  codesign --force --deep --options runtime --sign "$APPLE_DEVELOPER_ID" "$APP_BUNDLE"
else
  echo "Notice: APPLE_DEVELOPER_ID not set — producing unsigned local build"
fi

rm -f "$DMG_PATH"
hdiutil create -volname "$APP_NAME" -srcfolder "$APP_BUNDLE" -ov -format UDZO "$DMG_PATH" >/dev/null

if [[ -n "${APPLE_DEVELOPER_ID:-}" ]] && [[ -n "${APPLE_NOTARY_PROFILE:-}" ]]; then
  codesign --force --sign "$APPLE_DEVELOPER_ID" "$DMG_PATH"
  xcrun notarytool submit "$DMG_PATH" --keychain-profile "$APPLE_NOTARY_PROFILE" --wait
  xcrun stapler staple "$DMG_PATH"
fi

echo "Packaged app: $APP_BUNDLE"
echo "Packaged dmg: $DMG_PATH"
