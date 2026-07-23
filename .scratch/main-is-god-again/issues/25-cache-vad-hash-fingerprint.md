---
title: Stop hashing the VAD model on every launch
labels: [wayfinder:task]
status: open
assignee:
blocked_by: []
parent: MAP.md
---

## Question

`dest_needs_bundle_restore` (`services/bundled_models.rs:34-42`) runs a full SHA-256 over the VAD model file on every single startup (`lib.rs:611-613`, marked `hash_at_startup: true` because the file is "tiny — fine to pin every launch"). The file rarely changes. Should this be replaced with a cheap fingerprint (mtime + size) that only falls back to a full hash when the fingerprint changes, or is the VAD file small enough that this genuinely doesn't matter and the ticket should close as "not worth it"?

**Done when:** either (a) a mtime+size fingerprint cache is added so the hash only re-runs when the file's fingerprint changes, or (b) the ticket closes with a measured cost (e.g. actual hash time for the real VAD file) showing it's negligible and not worth the added complexity.

## Comments

- 2026-07-23: Ticketed from [[23-sequential-loading-habits-in-app-startup]]. Lowest-risk of the three startup tickets — pure caching, no behavior change to the integrity guarantee itself.
