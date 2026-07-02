# ScribeFloat Knowledge Query

Two knowledge files you can query:

| File | Contents |
|---|---|
| `ux.scribefloat.json` | UX & HCD Playbook — principles, interaction rules, typography, motion, accessibility, component guidance, checklists |
| `design-system.json` | Design System — color tokens, type scale, spacing, radius, component specs, surface layouts, copy rules |

Query them with `query.py`. Run from any directory — paths resolve relative to the script.

---

## Start here — get the map

```
python3 query.py ux toc        # playbook: all chapters and sections
python3 query.py ds toc        # design system: all sections and components
```

---

## Drill into the playbook

```
python3 query.py ux chapter Typography          # full chapter (partial name match)
python3 query.py ux chapter "Know Your User"    # chapter with spaces
python3 query.py ux section "Buttons & actions" # specific section (partial match)
python3 query.py ux section "litmus test"       # works on partial names
python3 query.py ux all                         # full playbook — use sparingly
```

---

## Drill into the design system

```
python3 query.py ds get components.button          # button variants + rules
python3 query.py ds get components.toggle          # any component
python3 query.py ds get tokens.colors.dark         # full dark color palette
python3 query.py ds get tokens.typography.scale    # all type scale entries
python3 query.py ds get tokens.spacing             # spacing scale
python3 query.py ds get rules.colors               # color usage rules + violations
python3 query.py ds get rules.typography           # type hierarchy rules
python3 query.py ds get rules.surfaces             # elevation model
python3 query.py ds get surfaces.scribeWindow      # Tailwind layout for main panel
python3 query.py ds get copy                       # tone, casing, copy rules
python3 query.py ds get meta                       # fonts, stack, color scheme
```

---

## Search when you don't know where something lives

```
python3 query.py search "waveform"          # both files at once
python3 query.py ux search "animation"      # playbook only
python3 query.py ds search "orange"         # design system only
python3 query.py ds search "focus"          # find all focus-related rules
```

---

## Progressive disclosure pattern

```
1. python3 query.py ux toc                  ← see all chapters + sections
2. python3 query.py ux chapter <name>       ← read a chapter
3. python3 query.py ux section <name>       ← read one section

1. python3 query.py ds toc                  ← see all tokens/components
2. python3 query.py ds get <path>           ← get the exact spec
```

Go deeper only if you need to. A chapter is usually enough context. `ux all` is expensive — avoid unless you need the whole document.
