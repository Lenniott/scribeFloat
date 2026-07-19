# Session capture hooks

Shared policy for Claude Code and Cursor: prompt once per session when capture may be needed.

## Triggers (any one)

- **15 user turns** since the last write to `docs/adr/`, `docs/agents/`, or `.scratch/`
- **Context ≥ 60%** (`preCompact` / `PreCompact`) or auto-compaction imminent

## Wiring

| Event | Script | Claude | Cursor local |
|-------|--------|--------|--------------|
| Stop | `stop.mjs` | ✓ | ✓ |
| User prompt | `on-prompt.mjs` | `UserPromptSubmit` | `beforeSubmitPrompt` |
| Doc write | `on-doc-write.mjs` | `PostToolUse` Write/Edit | `postToolUse` Write/StrReplace |
| Compaction | `on-compact.mjs` | `PreCompact` | `preCompact` |

State: `/tmp/session-capture-<session_id>/` (first 16 chars of `session_id` or `conversation_id`).

Cloud Cursor agents do not run `beforeSubmitPrompt` or `stop`; `postToolUse` + `preCompact` + AGENTS.md session capture policy cover those sessions.

## Dismissal

Reply **nothing to capture** on the next prompt, or ignore the nudge — both mark the session handled (`fired`).
