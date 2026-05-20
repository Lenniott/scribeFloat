#!/usr/bin/env bash
# Local dry-run of .github/workflows/release.yml macOS build job (sign + notarize + dmg).
# Requires macOS, Node 22+, Rust, and the same secrets as GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script only runs on macOS (matches the macos-14 Actions runner)."
  exit 1
fi

ENV_FILE="${RELEASE_ENV_FILE:-$ROOT/scripts/release-build.env.local}"
if [[ -f "$ENV_FILE" ]]; then
  echo "Loading env from $ENV_FILE"
  set -a
  # shellcheck source=/dev/null
  source "$ENV_FILE"
  set +a
else
  echo "No env file at $ENV_FILE"
  echo "Copy scripts/release-build.env.example → scripts/release-build.env.local and fill in."
  exit 1
fi

# Match release.yml Option A: do not set APPLE_SIGNING_IDENTITY.
unset APPLE_SIGNING_IDENTITY

if [[ -n "${APPLE_CERTIFICATE_PATH:-}" && -z "${APPLE_CERTIFICATE:-}" ]]; then
  if [[ ! -f "$APPLE_CERTIFICATE_PATH" ]]; then
    echo "APPLE_CERTIFICATE_PATH not found: $APPLE_CERTIFICATE_PATH"
    exit 1
  fi
  export APPLE_CERTIFICATE
  APPLE_CERTIFICATE="$(openssl base64 -A -in "$APPLE_CERTIFICATE_PATH")"
fi

missing=()
for var in APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; do
  if [[ -z "${!var:-}" ]]; then
    missing+=("$var")
  fi
done
if ((${#missing[@]} > 0)); then
  echo "Missing: ${missing[*]}"
  exit 1
fi

export TAURI_BUILD_TARGET=universal-apple-darwin
export CI=true

echo "==> rustup targets (same as Actions)"
rustup target add x86_64-apple-darwin aarch64-apple-darwin

echo "==> npm ci"
npm ci

echo "==> tauri build (same command as Actions macOS job)"
npm run tauri -- build --target "$TAURI_BUILD_TARGET"

DMG_GLOB="$ROOT/src-tauri/target/universal-apple-darwin/release/bundle/dmg/*.dmg"
APP_PATH="$ROOT/src-tauri/target/universal-apple-darwin/release/bundle/macos/ScribeFloat.app"

echo ""
echo "==> Artifacts"
ls -lh $DMG_GLOB

echo ""
echo "==> codesign (expect Developer ID Application for notarized releases)"
codesign -dv --verbose=2 "$APP_PATH" 2>&1 | rg -i 'authority|team|timestamp' || codesign -dv --verbose=2 "$APP_PATH"

echo ""
echo "Done. If this succeeded, the macOS Actions job should behave the same"
echo "(same secrets, same target, no APPLE_SIGNING_IDENTITY)."
