---
id: "0022"
title: Audit and document differences between ScribeController and DictateController
status: active
adr: ADR-0003
---

# Audit Scribe vs Dictate controller differences

Research/design task — no code changes.

Map every behavioural difference between `ScribeController` and `DictateController` to a config value:
- Audio durability
- Model selection
- Stop safeguard
- Output destination

Output: a decision on whether unification is safe to attempt and what the shared capture profile interface looks like. Write result as an ADR or update ADR-0003.
