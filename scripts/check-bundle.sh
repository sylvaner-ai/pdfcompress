#!/usr/bin/env bash
# Vérifie que l'application compresse avec le Ghostscript embarqué, Homebrew masqué :
# un petit programme C lié au moteur et à la libgs embarquée compresse un PDF de test.
# Usage : scripts/check-bundle.sh "chemin/vers/PDF Compress.app"
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$(cd "$1" && pwd)"
FRAMEWORKS="$APP/Contents/Frameworks"
WORK="$(mktemp -d)"

# Aucune dépendance ne doit pointer vers Homebrew.
if otool -L "$APP/Contents/MacOS/PDF Compress" "$FRAMEWORKS"/*.dylib | grep -E '/(opt/homebrew|usr/local)/'; then
  echo "Dépendance Homebrew restante" >&2
  exit 1
fi

# Le programme de test retrouve la libgs embarquée par le même chemin que l'application.
mkdir -p "$WORK/MacOS"
ln -s "$FRAMEWORKS" "$WORK/Frameworks"
LIBGS="$(find "$FRAMEWORKS" -name 'libgs*.dylib' | head -n 1)"
clang -I "$ROOT/engine/include" "$ROOT/engine/tests/selftest.c" \
  "$ROOT/engine/target/aarch64-apple-darwin/release/libpdfcompress_engine.a" "$LIBGS" \
  -o "$WORK/MacOS/selftest"

# PDF de test produit avec le Ghostscript de Homebrew, avant de le masquer.
gs -q -sDEVICE=pdfwrite -dNOPAUSE -dBATCH -dAutoFilterColorImages=false -dColorImageFilter=/FlateEncode \
  -sOutputFile="$WORK/test.pdf" "$ROOT/engine/tests/fixtures/sample.ps"

CELLAR="$(brew --cellar ghostscript)"
mv "$CELLAR" "$CELLAR.hidden"
trap 'mv "$CELLAR.hidden" "$CELLAR"' EXIT

"$WORK/MacOS/selftest" "$WORK/test.pdf" "$APP/Contents/Resources/gs"
