#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOCAL_DIR="$ROOT_DIR/assets/fonts/local"
ZIP_TMP="/tmp/MiSans-official.zip"
EXTRACT_TMP="/tmp/MiSans-extract"

mkdir -p "$LOCAL_DIR"
rm -rf "$EXTRACT_TMP" "$ZIP_TMP"

echo "Downloading official MiSans package from Xiaomi CDN..."
curl -fsSL "https://cdn.cnbj1.fds.api.mi-img.com/vipmlmodel/font/MiSans/MiSans.zip" -o "$ZIP_TMP"
unzip -q "$ZIP_TMP" -d "$EXTRACT_TMP"

for weight in Regular Medium Semibold; do
  found="$(find "$EXTRACT_TMP" -type f -iname "*MiSans*${weight}*.ttf" | head -n 1 || true)"
  if [[ -n "$found" ]]; then
    cp "$found" "$LOCAL_DIR/MiSans-${weight}.ttf"
    echo "Installed $LOCAL_DIR/MiSans-${weight}.ttf"
  fi
done

license_file="$(find "$EXTRACT_TMP" -type f \( -iname "*license*" -o -iname "*许可*" -o -iname "*.pdf" -o -iname "*.txt" \) | head -n 1 || true)"
if [[ -n "$license_file" ]]; then
  cp "$license_file" "$LOCAL_DIR/"
fi

rm -rf "$EXTRACT_TMP" "$ZIP_TMP"
echo "Done. Note: MiSans files in assets/fonts/local/ are gitignored for local/app-bundle embedding only."
