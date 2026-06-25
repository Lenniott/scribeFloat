#!/usr/bin/env bash
# Downloads the speaker embedding model used by voice-probe.
# Run this once from tools/voice-probe/ before `cargo run`.
set -e

MODEL="3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx"
URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/${MODEL}"

if [ -f "./${MODEL}" ]; then
  echo "Model already present: ${MODEL}"
else
  echo "Downloading ${MODEL} (~25 MB) ..."
  curl -L -o "${MODEL}" "${URL}"
  echo "Done."
fi

echo ""
echo "Next steps:"
echo "  1. Add WAVs of your own voice to  test-audio/me/"
echo "  2. Add WAVs of other speakers to  test-audio/other/"
echo "  3. Run:  cargo run"
echo ""
echo "WAV requirements: 16 kHz, mono, at least 2 s of speech per file."
echo "Record multiple clips per mic / distance combo for better results."
