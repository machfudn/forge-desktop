#!/usr/bin/env bash
set -euo pipefail

DIST_DIR="${1:-src-tauri/target/release/bundle}"
KEY_ID="${GPG_KEY_ID:-}"
PUBLIC_KEY="packaging/keys/forge-desktop-release-public.asc"

if [ -z "$KEY_ID" ]; then
  echo "GPG_KEY_ID tidak diset. Melewati penandatanganan Linux."
  exit 0
fi

echo "Mencari artefak Linux di $DIST_DIR..."
mapfile -t ARTIFACTS < <(find "$DIST_DIR" -type f \( -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" \) | sort)

if [ ${#ARTIFACTS[@]} -eq 0 ]; then
  echo "Tidak ada artefak Linux (.deb, .rpm, .AppImage) ditemukan."
  exit 1
fi

CHECKSUM_FILE="$DIST_DIR/SHA256SUMS"
SIG_FILE="$CHECKSUM_FILE.asc"

echo "Membuat SHA256SUMS..."
(
  cd "$DIST_DIR"
  find . -maxdepth 3 -type f \( -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" \) -exec sha256sum {} + > SHA256SUMS
)

echo "Menandatangani SHA256SUMS dengan GPG key $KEY_ID..."
gpg --batch --yes --armor \
  --local-user "$KEY_ID" \
  --detach-sign \
  --output "$SIG_FILE" \
  "$CHECKSUM_FILE"

echo "Memverifikasi tanda tangan..."
if [ -f "$PUBLIC_KEY" ]; then
  gpg --batch --verify "$SIG_FILE" "$CHECKSUM_FILE"
  echo "Tanda tangan GPG terverifikasi sukses!"
fi
