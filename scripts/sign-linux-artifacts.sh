#!/usr/bin/env bash
set -euo pipefail

DIST_DIR="${1:-src-tauri/target/release/bundle}"
KEY_ID="${GPG_KEY_ID:-}"
PASSPHRASE="${GPG_PASSPHRASE:-}"
PUBLIC_KEY="packaging/keys/forge-desktop-release-public.asc"

echo "Mencari artefak rilis di $DIST_DIR..."
mapfile -t ARTIFACTS < <(
  find "$DIST_DIR" -maxdepth 4 -type f \
    \( -name "*.exe" -o -name "*.msi" -o -name "*.zip" -o -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" -o -name "*.dmg" -o -name "*.tar.gz" \) \
    | sort
)

if [ ${#ARTIFACTS[@]} -eq 0 ]; then
  echo "Tidak ada artefak rilis ditemukan di $DIST_DIR."
  exit 1
fi

CHECKSUM_FILE="$DIST_DIR/SHA256SUMS"
SIG_FILE="$CHECKSUM_FILE.asc"

echo "Membuat SHA256SUMS untuk ${#ARTIFACTS[@]} artefak..."
(
  cd "$DIST_DIR"
  find . -maxdepth 4 -type f \
    \( -name "*.exe" -o -name "*.msi" -o -name "*.zip" -o -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" -o -name "*.dmg" -o -name "*.tar.gz" \) \
    -print0 | LC_ALL=C sort -z | xargs -0 sha256sum | sed -E 's|^([0-9a-f]{64} [ *])\./|\1|' > SHA256SUMS
)

if [ -z "$KEY_ID" ]; then
  echo "GPG_KEY_ID tidak diset. SHA256SUMS dibuat tanpa tanda tangan GPG."
  exit 0
fi

echo "Menandatangani SHA256SUMS dengan GPG key $KEY_ID..."
if [ -n "$PASSPHRASE" ]; then
  printf '%s' "$PASSPHRASE" | gpg --batch --yes --armor \
    --pinentry-mode loopback --passphrase-fd 0 \
    --local-user "$KEY_ID" \
    --detach-sign \
    --output "$SIG_FILE" \
    "$CHECKSUM_FILE"
else
  gpg --batch --yes --armor \
    --pinentry-mode loopback --passphrase "" \
    --local-user "$KEY_ID" \
    --detach-sign \
    --output "$SIG_FILE" \
    "$CHECKSUM_FILE"
fi

echo "Memverifikasi tanda tangan..."
if [ -f "$PUBLIC_KEY" ]; then
  gpg --batch --import "$PUBLIC_KEY" || true
fi
gpg --batch --verify "$SIG_FILE" "$CHECKSUM_FILE"
echo "Tanda tangan GPG terverifikasi sukses!"
