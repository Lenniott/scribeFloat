---
id: "0052"
title: Float inference backend — InferenceService, FloatController, and IPC commands
status: active
exploration: design-brain-prd.md
---

# Float inference backend — InferenceService, FloatController, and IPC commands

The Rust backend for Float's HTTP inference layer has been implemented on branch
`claude/ollama-http-local-perf-xrh30r` and merged. This story documents what was
built and what remains.

## What was built

- **`src/services/inference.rs`** — `InferenceService` wraps a `reqwest::Client` (15 s timeout) and exposes `list_models(provider, endpoint, api_key)`. Provider-aware:
  - Ollama: `GET /api/tags` → `{models:[{name}]}`
  - OpenAI / Custom: `GET /v1/models` with optional `Authorization: Bearer` header; embed/tts/dall-e/whisper models filtered out
  - Anthropic: `GET /v1/models` with `x-api-key` + `anthropic-version: 2023-06-01` headers
- **`src/controllers/float.rs`** — `FloatController` with `get_config`, `set_config`, `clear_api_key`, `list_models`, `test_connection`
- **`src/commands/float.rs`** — 5 IPC commands: `float_get_config`, `float_set_config`, `float_clear_api_key`, `float_list_models`, `float_test_connection`
- **`src/types.rs`** — `FloatProvider`, `FloatConfig`, `FloatModelInfo` types; 4 new `Config` fields (`float_provider`, `float_endpoint_url`, `float_api_key`, `float_model`) all `#[serde(default)]`

## What remains

- **Settings UI** — frontend panel for the user to select provider, enter endpoint URL, store API key, pick a model, and run a connection test. Blocked on UX discussion: onboarding flow, determinism of inputs, what happens when no model is configured (skip Float silently vs. surface a prompt).
- **Float enrichment queue + flow engine** — `EnrichmentQueue`, `FlowEngine`, `Chunker`, `LayerRegistry` described in `design-brain-prd.md`. The inference layer is the foundation; the queue and flow engine are the next backend step.

## Notes

- API key is never sent to the frontend; `FloatConfig.has_api_key: bool` only.
- `float_model: None` means Float is not ready — on-creation flow must not run until a model is selected.
- Queue concurrency is 1 (Ollama serialises internally; keeps implementation simple).
