# Engineering Docs

Focused reference files for agents doing building sessions. Each file covers one domain — load only the file relevant to your task.

**Update a file in the same session you change the behaviour it describes.**

---

## What's here

| File | Read when | Update when |
|------|-----------|-------------|
| [layer-rules.md](layer-rules.md) | Adding a feature, adding an IPC command, deciding which layer a change belongs to | You change the call chain, add a service, or change ownership rules |
| [async-rules.md](async-rules.md) | Touching controller threading, Whisper inference paths, audio callbacks, or state machines | You add a new controller state, change async patterns, or discover a new threading constraint |
| [platform-rules.md](platform-rules.md) | Touching macOS threading, audio drain, paste behaviour, or anything in `platform/` | You discover a new main-thread constraint, change the drain logic, or add a platform adapter |
| [debugging.md](debugging.md) | Investigating a bug — find the right file to start in | You find a symptom not in the table, or a starting point turns out to be wrong |
| [config-rules.md](config-rules.md) | Adding or changing a field in `Config` (`types.rs`) | Config schema or save behaviour changes |

---

## Schema for engineering docs

Each file in this folder follows the same structure:

```
# Title

> Load this when [specific trigger — one line].

---

## [Section]

[Content — rules, not prose. Short. Imperative.]
```

No narrative. No history. Just what an agent needs to act correctly.
