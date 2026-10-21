#!/usr/bin/env bash
# Embarque Ghostscript dans l'application construite : libgs et ses dépendances dans
# Contents/Frameworks, ressources PostScript (Init, polices, profils ICC) dans Contents/Resources/gs.
# Usage : scripts/bundle-ghostscript.sh "chemin/vers/PDF Compress.app"
# Prérequis : brew install ghostscript dylibbundler
set -euo pipefail

APP="$1"
BIN="$APP/Contents/MacOS/PDF Compress"
FRAMEWORKS="$APP/Contents/Frameworks"
DEST="$APP/Contents/Resources/gs"
PREFIX="$(brew --prefix ghostscript)"

rm -rf "$DEST"
mkdir -p "$FRAMEWORKS" "$DEST"

# On part de gs_init.ps pour ne pas dépendre de l'organisation exacte du paquet Homebrew.
INIT_FILE="$(find -L "$PREFIX/" -name gs_init.ps -path '*/Resource/Init/*' | head -n 1)"
if [ -z "$INIT_FILE" ]; then
  echo "gs_init.ps introuvable dans $PREFIX" >&2
  exit 1
fi
RESOURCE_DIR="$(dirname "$(dirname "$INIT_FILE")")"
VERSION_DIR="$(dirname "$RESOURCE_DIR")"
echo "Ressources Ghostscript : $VERSION_DIR"
cp -RL "$RESOURCE_DIR" "$DEST/Resource"
for d in lib iccprofiles; do
  if [ -d "$VERSION_DIR/$d" ]; then cp -RL "$VERSION_DIR/$d" "$DEST/"; fi
done
if [ -d "$VERSION_DIR/../fonts" ]; then cp -RL "$VERSION_DIR/../fonts" "$DEST/"; fi

# Copie libgs et ses dépendances, et réécrit leurs chemins relativement à l'exécutable.
dylibbundler -of -b -x "$BIN" -d "$FRAMEWORKS" -p "@executable_path/../Frameworks/" -s "$(brew --prefix)/lib" -s "$PREFIX/lib"

# Signature ad hoc provisoire (obligatoire sur puce Apple) ; la signature Developer ID la remplace.
codesign --force --sign - "$FRAMEWORKS"/*.dylib
codesign --force --sign - "$APP"

echo "Ghostscript embarqué ($(du -sh "$FRAMEWORKS" | cut -f1) de bibliothèques, $(du -sh "$DEST" | cut -f1) de ressources)"
