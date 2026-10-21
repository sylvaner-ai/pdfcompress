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

# dyld refuse de charger un binaire dont un LC_RPATH apparaît deux fois.
for f in "$APP/Contents/MacOS/PDF Compress" "$FRAMEWORKS"/*.dylib; do
  DUP="$(otool -l "$f" | awk '/cmd LC_RPATH/ {getline; getline; print $2}' | sort | uniq -d)"
  if [ -n "$DUP" ]; then
    echo "LC_RPATH en double dans $f : $DUP" >&2
    exit 1
  fi
done

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

# L'application elle-même doit démarrer (chargement par dyld, signature, interface).
"$APP/Contents/MacOS/PDF Compress" > "$WORK/app.log" 2>&1 &
PID=$!
sleep 8
if ! kill -0 "$PID" 2>/dev/null; then
  echo "L'application s'est arrêtée au démarrage :" >&2
  cat "$WORK/app.log" >&2
  exit 1
fi
kill "$PID"
echo "L'application démarre."
