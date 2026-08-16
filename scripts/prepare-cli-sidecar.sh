#!/usr/bin/env bash
# Build the `scribefloat-cli` binary and stage it as a Tauri externalBin sidecar
# (see tauri.conf.json bundle.externalBin) so it ships inside the app bundle/installer.
# Must run before `cargo tauri build` / `cargo tauri dev`: Tauri's build script validates
# that triple-suffixed externalBin files already exist (same requirement as
# bundle.resources — see fetch-bundled-models.sh).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SRC_TAURI="$ROOT_DIR/src-tauri"

# Allow CI to override for cross-compiled/`--target` builds; default to the host triple.
TARGET="${TAURI_ENV_TARGET_TRIPLE:-${1:-}}"
if [[ -z "$TARGET" ]]; then
  TARGET="$(rustc -vV | awk '/^host:/ {print $2}')"
fi

EXT=""
[[ "$TARGET" == *windows* ]] && EXT=".exe"

echo "Building scribefloat-cli for $TARGET..."
cargo build --release --manifest-path "$SRC_TAURI/Cargo.toml" --bin scribefloat-cli --target "$TARGET"

BUNDLE_DIR="$SRC_TAURI/binaries"
mkdir -p "$BUNDLE_DIR"
SRC_BIN="$SRC_TAURI/target/$TARGET/release/scribefloat-cli${EXT}"
DEST_BIN="$BUNDLE_DIR/scribefloat-cli-${TARGET}${EXT}"

# Copy only when changed so `tauri dev` doesn't retrigger its file watcher every rebuild.
if [[ -f "$DEST_BIN" ]] && cmp -s "$SRC_BIN" "$DEST_BIN"; then
  exit 0
fi
cp "$SRC_BIN" "$DEST_BIN"
chmod +x "$DEST_BIN"
