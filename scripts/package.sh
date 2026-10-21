#!/usr/bin/env bash
# Signe l'application, crée le .dmg et le fait notariser par Apple.
# Usage : scripts/package.sh "chemin/vers/PDF Compress.app" sortie.dmg
#
# Avec les variables suivantes (secrets GitHub, voir docs/signature.md), l'application est
# signée Developer ID puis notarisée ; sans elles, elle reste signée ad hoc (non notarisée) :
#   MACOS_CERTIFICATE_P12, MACOS_CERTIFICATE_PASSWORD, APPLE_TEAM_ID,
#   APPSTORE_API_KEY_ID, APPSTORE_API_ISSUER_ID, APPSTORE_API_KEY_P8
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$1"
DMG="$2"
ENTITLEMENTS="$ROOT/app/PDFCompress/PDFCompress.entitlements"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

make_dmg() {
  mkdir -p "$WORK/dmg"
  cp -R "$APP" "$WORK/dmg/"
  ln -s /Applications "$WORK/dmg/Applications"
  rm -f "$DMG"
  hdiutil create -volname "PDF Compress" -srcfolder "$WORK/dmg" -ov -format UDZO "$DMG"
}

if [ -z "${MACOS_CERTIFICATE_P12:-}" ]; then
  echo "::warning::Secrets de signature absents : application signée ad hoc, non notarisée."
  make_dmg
  exit 0
fi

: "${MACOS_CERTIFICATE_PASSWORD:?}" "${APPLE_TEAM_ID:?}" "${APPSTORE_API_KEY_ID:?}" "${APPSTORE_API_ISSUER_ID:?}" "${APPSTORE_API_KEY_P8:?}"

# Trousseau temporaire contenant le certificat Developer ID.
KEYCHAIN="$WORK/signature.keychain-db"
KEYCHAIN_PASSWORD="$(openssl rand -hex 24)"
security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
security set-keychain-settings -lut 3600 "$KEYCHAIN"
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
printf '%s' "$MACOS_CERTIFICATE_P12" | base64 --decode > "$WORK/certificat.p12"
security import "$WORK/certificat.p12" -k "$KEYCHAIN" -P "$MACOS_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
rm -f "$WORK/certificat.p12"
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN" > /dev/null
security list-keychains -d user -s "$KEYCHAIN" $(security list-keychains -d user | tr -d '"')

IDENTITY="$(security find-identity -v -p codesigning "$KEYCHAIN" | grep "Developer ID Application" | grep "($APPLE_TEAM_ID)" | head -n 1 | awk '{print $2}')"
if [ -z "$IDENTITY" ]; then
  echo "Aucun certificat « Developer ID Application » de l'équipe $APPLE_TEAM_ID dans MACOS_CERTIFICATE_P12." >&2
  exit 1
fi

# Signature de l'intérieur vers l'extérieur, avec Hardened Runtime et horodatage (exigés par la notarisation).
sign() { codesign --force --timestamp --options runtime --sign "$IDENTITY" "$@"; }
find "$APP/Contents/Frameworks" -name '*.dylib' -print0 | while IFS= read -r -d '' lib; do sign "$lib"; done
sign --entitlements "$ENTITLEMENTS" "$APP"
codesign --verify --strict --deep --verbose=2 "$APP"

make_dmg
sign "$DMG"

# Notarisation avec une clé API App Store Connect, puis agrafage du ticket.
printf '%s' "$APPSTORE_API_KEY_P8" | base64 --decode > "$WORK/AuthKey.p8"
xcrun notarytool submit "$DMG" --key "$WORK/AuthKey.p8" --key-id "$APPSTORE_API_KEY_ID" --issuer "$APPSTORE_API_ISSUER_ID" --wait --timeout 30m
xcrun stapler staple "$DMG"
spctl --assess --type open --context context:primary-signature --verbose "$DMG"
echo "Application signée et notarisée : $DMG"
