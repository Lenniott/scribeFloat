#!/usr/bin/env bash
# Download the models bundled into release builds (see tauri.conf.json bundle.resources).
# Run before `cargo tauri build`. Idempotent: existing files that pass their checksum are kept.
# Dev builds work without these files — model-dependent features report the missing file.
set -euo pipefail

DEST_DIR="$(cd "$(dirname "$0")/.." && pwd)/src-tauri/bundled-models"
mkdir -p "$DEST_DIR"

# name | url | sha256 (empty = unverified; the voiceprint release asset publishes no checksum)
MODELS=(
  "ggml-small.en-q5_1.bin|https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en-q5_1.bin|bfdff4894dcb76bbf647d56263ea2a96645423f1669176f4844a1bf8e478ad30"
  "ggml-silero-v6.2.0.bin|https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v6.2.0.bin|2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987"
  "3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx|https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx|"
  "diar_streaming_sortformer_4spk-v2.onnx|https://huggingface.co/altunenes/parakeet-rs/resolve/main/diar_streaming_sortformer_4spk-v2.onnx|cc520901a8cc25a8d7f7c2c8561a465709b67dd4f1df0572a97530087f3fbc73"
)

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

for entry in "${MODELS[@]}"; do
  IFS='|' read -r name url sha <<<"$entry"
  dest="$DEST_DIR/$name"

  if [[ -s "$dest" ]]; then
    if [[ -z "$sha" ]] || [[ "$(sha256_of "$dest")" == "$sha" ]]; then
      echo "ok: $name (already present)"
      continue
    fi
    echo "checksum mismatch for existing $name — re-downloading"
    rm -f "$dest"
  fi

  echo "downloading $name ..."
  curl -fL --retry 3 --progress-bar -o "$dest.tmp" "$url"

  if [[ -n "$sha" ]]; then
    actual="$(sha256_of "$dest.tmp")"
    if [[ "$actual" != "$sha" ]]; then
      rm -f "$dest.tmp"
      echo "error: checksum mismatch for $name (expected $sha, got $actual)" >&2
      exit 1
    fi
  fi

  mv "$dest.tmp" "$dest"
  echo "ok: $name"
done

echo "bundled models ready in $DEST_DIR"
