#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-0.1.0}"
ARCH="$(uname -m)"
DIST_DIR="$ROOT_DIR/dist"
STAGE_DIR="$DIST_DIR/lingxi-t3-native-$VERSION-linux-$ARCH"
ARCHIVE="$DIST_DIR/LingXiT3-Native-$VERSION-linux-$ARCH.tar.gz"

cd "$ROOT_DIR"
cargo build --release --bin lingxi-t3-native

rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR/bin" "$STAGE_DIR/share/applications" "$STAGE_DIR/share/icons/hicolor/512x512/apps"
cp "$ROOT_DIR/target/release/lingxi-t3-native" "$STAGE_DIR/bin/"
cp "$ROOT_DIR/assets/icons/icon-512.png" "$STAGE_DIR/share/icons/hicolor/512x512/apps/lingxi-t3-native.png"

cat > "$STAGE_DIR/share/applications/lingxi-t3-native.desktop" <<EOF
[Desktop Entry]
Name=LingXi T3 Native
Exec=lingxi-t3-native %U
Icon=lingxi-t3-native
Type=Application
Categories=Development;
MimeType=x-scheme-handler/lingxit3-native;
EOF

tar -C "$DIST_DIR" -czf "$ARCHIVE" "$(basename "$STAGE_DIR")"
echo "Packaged Linux archive: $ARCHIVE"
