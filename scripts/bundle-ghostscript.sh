#!/usr/bin/env bash
# Prépare une copie autonome de Ghostscript (binaire, bibliothèques et ressources)
# dans src-tauri/gs-dist, pour l'embarquer dans l'application macOS.
# Prérequis : brew install ghostscript dylibbundler
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/src-tauri/gs-dist"
PREFIX="$(brew --prefix ghostscript)"
BREW_LIB="$(brew --prefix)/lib"

rm -rf "$DEST"
mkdir -p "$DEST/bin" "$DEST/lib" "$DEST/share"

cp -L "$PREFIX/bin/gs" "$DEST/bin/gs"
chmod 755 "$DEST/bin/gs"

# Ressources PostScript (Init, polices, profils ICC) : Ghostscript les trouve via GS_LIB.
VERSION_DIR="$(find "$PREFIX/share/ghostscript" -mindepth 1 -maxdepth 1 -type d -name '[0-9]*' | head -n 1)"
for d in Resource lib iccprofiles; do
  if [ -d "$VERSION_DIR/$d" ]; then cp -R "$VERSION_DIR/$d" "$DEST/share/"; fi
done
if [ -d "$PREFIX/share/ghostscript/fonts" ]; then cp -R "$PREFIX/share/ghostscript/fonts" "$DEST/share/"; fi

# Copie les bibliothèques dynamiques et réécrit leurs chemins relativement au binaire.
dylibbundler -of -cd -b -x "$DEST/bin/gs" -d "$DEST/lib" -p "@executable_path/../lib/" -s "$BREW_LIB" -s "$PREFIX/lib"

# Les binaires modifiés doivent être re-signés (signature ad hoc) pour s'exécuter sur Apple Silicon.
codesign --force --sign - "$DEST"/lib/*.dylib "$DEST/bin/gs"

echo "Ghostscript $("$DEST/bin/gs" --version) prêt dans $DEST ($(du -sh "$DEST" | cut -f1))"
