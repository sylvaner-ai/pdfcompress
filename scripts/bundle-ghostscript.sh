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
# On part de gs_init.ps pour ne pas dépendre de l'organisation exacte du paquet Homebrew.
INIT_FILE="$(find -L "$PREFIX/" -name gs_init.ps -path '*/Resource/Init/*' | head -n 1)"
if [ -z "$INIT_FILE" ]; then
  echo "gs_init.ps introuvable dans $PREFIX" >&2
  exit 1
fi
RESOURCE_DIR="$(dirname "$(dirname "$INIT_FILE")")"
VERSION_DIR="$(dirname "$RESOURCE_DIR")"
echo "Ressources Ghostscript : $VERSION_DIR"
cp -RL "$RESOURCE_DIR" "$DEST/share/Resource"
for d in lib iccprofiles; do
  if [ -d "$VERSION_DIR/$d" ]; then cp -RL "$VERSION_DIR/$d" "$DEST/share/"; fi
done
if [ -d "$VERSION_DIR/../fonts" ]; then cp -RL "$VERSION_DIR/../fonts" "$DEST/share/"; fi

# Copie les bibliothèques dynamiques et réécrit leurs chemins relativement au binaire.
dylibbundler -of -cd -b -x "$DEST/bin/gs" -d "$DEST/lib" -p "@executable_path/../lib/" -s "$BREW_LIB" -s "$PREFIX/lib"

# Les binaires modifiés doivent être re-signés (signature ad hoc) pour s'exécuter sur Apple Silicon.
codesign --force --sign - "$DEST"/lib/*.dylib "$DEST/bin/gs"

# Vérifie que le binaire démarre avec les ressources copiées (même GS_LIB que l'application).
S="$DEST/share"
GS_LIB="$S/Resource/Init:$S/lib:$S/Resource/Font:$S/iccprofiles:$S/fonts" \
  "$DEST/bin/gs" -q -dNODISPLAY -dBATCH -dNOPAUSE -c quit

echo "Ghostscript $("$DEST/bin/gs" --version) prêt dans $DEST ($(du -sh "$DEST" | cut -f1))"
