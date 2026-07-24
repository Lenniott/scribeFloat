---
title: "Triage: CSP: style-src left unmodified by Tauri"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Question

Read the "CSP: style-src left unmodified by Tauri" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) (search for that heading). Decide: is this **Now** (fix it in this effort — state the concrete change, then make it) or **Later** (state why, and whether it's big enough to need its own future wayfinder)?

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
