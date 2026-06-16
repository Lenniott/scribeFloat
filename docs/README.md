# Docs index (for agents)

Short-lived notes and UI specs live here. **Behaviour and architecture** stay in `context/` — update those when workflows or ownership change.

| Doc | When to read |
|-----|----------------|
| [../context/README.md](../context/README.md) | First — reading order + behaviour quick reference |
| [../CLAUDE.md](../CLAUDE.md) | Build, layers, do-not-regress (Scribe start, History UI), debugging table |
| [../context/action-flows.md](../context/action-flows.md) | Step-by-step user flows (source of truth) |
| [onboarding.md](onboarding.md) | Current 5-step onboarding implementation notes |
| [history-ui-review.md](history-ui-review.md) | Any History list/detail/card/footer work |
| [typography-audit.md](typography-audit.md) | Typography consistency pass — inline vs `sf-*` role classes |
| [color-audit.md](color-audit.md) | Color consistency pass — fg opacity ladder vs semantic tokens |
| [../.cursor/skills/ui-enforcement/SKILL.md](../.cursor/skills/ui-enforcement/SKILL.md) | UI enforcement skill (typography + future aspects) |
| [../.cursor/rules/ui-enforcement.mdc](../.cursor/rules/ui-enforcement.mdc) | Auto-attached Cursor rule for `src/**/*.svelte` and `src/app.css` |
| [../.cursor/hooks.json](../.cursor/hooks.json) | Agent hooks — `check:ds` + typography warnings after frontend edits |
| [backlog.md](backlog.md) | Deferred / follow-up items |
| [design-brain-prd.md](design-brain-prd.md) | Proposal: local-LLM enrichment engine (Schema/Step/Flow) — pre-spike, nothing built yet |

If `context/` and code disagree, fix the doc or confirm the code change was intentional.
