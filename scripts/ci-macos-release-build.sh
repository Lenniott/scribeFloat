#!/usr/bin/env bash
# Local dry-run of .github/workflows/release.yml macOS build jobs (sign + notarize + dmg).
# Requires macOS, Node 22+, Rust, and the same secrets as GitHub Actions.
#
# CI builds two native slices (not universal):
#   TAURI_BUILD_TARGET=aarch64-apple-darwin  (default — Apple Silicon / macos-14 runner)
#   TAURI_BUILD_TARGET=x86_64-apple-darwin     (Intel — macos-15-intel runner + AVX2 CMAKE_ARGS)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script only runs on macOS."
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

export TAURI_BUILD_TARGET="${TAURI_BUILD_TARGET:-aarch64-apple-darwin}"
export CI=true

if [[ "${TAURI_BUILD_TARGET}" == "x86_64-apple-darwin" ]]; then
  export CMAKE_ARGS="-DGGML_ACCELERATE=ON -DGGML_BLAS=ON -DGGML_BLAS_VENDOR=Apple -DGGML_AVX2=ON -DGGML_FMA=ON -DGGML_F16C=ON"
fi

echo "==> tauri build target: ${TAURI_BUILD_TARGET}"

echo "==> fetch bundled models"
bash "$ROOT/scripts/fetch-bundled-models.sh"

echo "==> npm ci"
npm ci

echo "==> tauri build (same command as Actions macOS job)"
npm run tauri -- build --target "$TAURI_BUILD_TARGET"

DMG_GLOB="$ROOT/src-tauri/target/${TAURI_BUILD_TARGET}/release/bundle/dmg/*.dmg"
APP_PATH="$ROOT/src-tauri/target/${TAURI_BUILD_TARGET}/release/bundle/macos/ScribeFloat.app"

echo ""
echo "==> Artifacts"
ls -lh $DMG_GLOB

echo ""
echo "==> codesign (expect Developer ID Application for notarized releases)"
codesign -dv --verbose=2 "$APP_PATH" 2>&1 | rg -i 'authority|team|timestamp' || codesign -dv --verbose=2 "$APP_PATH"

echo ""
echo "Done. Re-run with TAURI_BUILD_TARGET=x86_64-apple-darwin to match the Intel CI job."
