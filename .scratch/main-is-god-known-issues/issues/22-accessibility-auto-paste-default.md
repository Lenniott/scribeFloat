---
title: "Triage: Accessibility + auto-paste on by default"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Question

Read the "Accessibility + auto-paste on by default" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) (search for that heading). Decide: is this **Now** (fix it in this effort — state the concrete change, then make it) or **Later** (state why, and whether it's big enough to need its own future wayfinder)?

## Findings

- **Default is set in the config struct's `Default` impl**: `src-tauri/src/types.rs:99` declares `pub dictate_auto_paste: bool` on the settings/config struct, and `src-tauri/src/types.rs:137` sets `dictate_auto_paste: true` as the default value used when config is first created (new install / no persisted config yet).
- **Runtime get/set**: `SettingsController::get_dictate_auto_paste` / `set_dictate_auto_paste` (`src-tauri/src/controllers/settings.rs:285-293`) are thin read/persist wrappers around `cfg.dictate_auto_paste` — user can already toggle this off via Settings; the ticket is purely about the *initial default* for new users/onboarding, not about whether the toggle exists (it does).
- **Behavior when on**: paste happens via `OutputService::paste_text` (`src-tauri/src/services/output/mod.rs:191-193`), which shells out to `crate::platform::paste_impl::paste_text()` — a simulated Cmd/Ctrl+V into the currently focused application, requiring Accessibility permission on macOS. This is documented product behavior (per the map note), not a bug — it's a UX/product trade-off (frictionless dictation vs. stricter default posture where the user must opt in to auto-paste and Accessibility access).
- **No code change needed to investigate** — this is a single boolean default (`types.rs:137`) and would be a one-line flip (`true` → `false`) if the product decision is made to default it off; no confinement/validation logic is involved since this isn't a path-safety issue like tickets 18/19/21, it's a permission-scope/UX default.
- **Size estimate**: Trivial if the decision is "flip the default" (one line + likely an onboarding copy/flow update to explain why paste isn't automatic yet). The map already flags this correctly as a "Product call," not a security defect — current behavior is a conscious, documented trade-off rather than an oversight.
