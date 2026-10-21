#!/usr/bin/env bash
# Vérifie que le Ghostscript embarqué dans l'application fonctionne sans Homebrew.
# Usage : scripts/check-bundled-gs.sh "chemin/vers/PDF Compress.app"
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$1"
GS_ROOT="$APP/Contents/Resources/gs"
GS="$GS_ROOT/bin/gs"
SHARE="$GS_ROOT/share"
WORK="$(mktemp -d)"

test -x "$GS"

# Aucune dépendance ne doit pointer vers Homebrew.
if otool -L "$GS" "$GS_ROOT"/lib/*.dylib | grep -E '/(opt/homebrew|usr/local)/'; then
  echo "Dépendance Homebrew restante" >&2
  exit 1
fi

# Masque l'installation Homebrew de Ghostscript le temps du test.
CELLAR="$(brew --cellar ghostscript)"
mv "$CELLAR" "$CELLAR.hidden"
trap 'mv "$CELLAR.hidden" "$CELLAR"' EXIT

export GS_LIB="$SHARE/Resource/Init:$SHARE/lib:$SHARE/Resource/Font:$SHARE/iccprofiles:$SHARE/fonts"
"$GS" -q -sDEVICE=pdfwrite -dNOPAUSE -dBATCH -dAutoFilterColorImages=false -dColorImageFilter=/FlateEncode \
  -sOutputFile="$WORK/in.pdf" "$ROOT/tests/fixtures/sample.ps"
"$GS" -q -sDEVICE=pdfwrite -dNOPAUSE -dBATCH -dSAFER -dPDFSETTINGS=/ebook -sOutputFile="$WORK/out.pdf" "$WORK/in.pdf"

BEFORE=$(stat -f%z "$WORK/in.pdf")
AFTER=$(stat -f%z "$WORK/out.pdf")
echo "Ghostscript embarqué $("$GS" --version) : $BEFORE -> $AFTER octets"
test "$AFTER" -gt 0 && test "$AFTER" -lt "$BEFORE"
