#!/usr/bin/env bash
# Compile le moteur Rust (bibliothèque statique) pour l'application, sur Mac à puce Apple.
# Lancé automatiquement par Xcode avant la compilation de l'application.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# Xcode lance les scripts avec un PATH réduit.
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-14.0}"
export GS_LIB_DIR="${GS_LIB_DIR:-$(brew --prefix)/lib}"

cargo build --release --target aarch64-apple-darwin --manifest-path "$ROOT/engine/Cargo.toml"
