---
title: "Triage: CSP: style-src left unmodified by Tauri"
labels: [wayfinder:grilling]
status: closed
assignee:
blocked_by: []
parent: ../MAP.md
---

## Issue

`tauri.conf.json` deliberately disables Tauri's asset-CSP nonce/hash injection for `style-src` only (via `dangerousDisableAssetCspModification: ["style-src"]`), relying instead on a static `'self' 'unsafe-inline'` value — this is intentional (needed for CodeMirror/skeleton runtime style injection). `script-src` is untouched and still gets Tauri's normal hardening.

## Question

Read the "CSP: style-src left unmodified by Tauri" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) for full context. No action needed — informational, current config is intentional.

## Resolution

**Verified 2026-07-29.** `tauri.conf.json:15-16` unchanged: `dangerousDisableAssetCspModification: ["style-src"]` scopes only to style-src, `script-src 'self'` still gets Tauri's normal nonce/hash hardening. No code change needed.

## Findings

- Confirmed present today in `src-tauri/tauri.conf.json` (lines 14-17):
  ```
  "csp": "default-src 'self' asset: https://asset.localhost; script-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self' asset: https://asset.localhost data:; img-src 'self' asset: https://asset.localhost data:; connect-src 'self' ipc: http://ipc.localhost",
  "dangerousDisableAssetCspModification": ["style-src"]
  ```
- The disable list scopes only to `["style-src"]` — Tauri's asset-CSP nonce/hash injection is skipped for `style-src` alone.
- `script-src 'self'` is untouched by this flag and remains subject to Tauri's normal CSP modification (nonce/hash injection for scripts still applies). Only style tags/attributes lose Tauri's injected protection; script execution policy is unaffected.
- Net effect: `style-src` relies on the static `'self' 'unsafe-inline'` value in the conf file (already permissive for inline styles, which is why CodeMirror/skeleton runtime style injection works), while `script-src 'self'` keeps whatever nonce/hash hardening Tauri applies at build time.
- No changes made — this is confirmed already-resolved/informational per the existing instruction not to remove the flag.
