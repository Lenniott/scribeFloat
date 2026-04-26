# Design System Specification: The Precision Instrument
 
## 1. Overview & Creative North Star
The Creative North Star for this design system is **"The Precision Instrument."** 
 
This system rejects the "friendly" softness of modern SaaS in favor of a high-fidelity, industrial aesthetic. It is inspired by the modularity of rack-mounted engineering gear and the stark clarity of laboratory equipment. We achieve a premium feel not through decoration, but through extreme intentionality, rigorous spatial relationships, and a rejection of standard UI tropes like rounded corners and decorative borders.
 
To break the "template" look, we utilize **Functional Asymmetry**. Layouts should feel like a technical schematic—where data density is high, and every pixel serves a mechanical purpose. We favor wide gutters, stark typographic scaling, and a "layered-machine" approach to depth.
 
---
 
## 2. Colors: The High-Contrast Laboratory
The palette is rooted in a "Void" state—using deep blacks and charcoals to minimize eye strain during long technical transcription sessions, punctuated by a high-visibility Technical Orange.
 
### Core Tokens
- **Background:** `#0e0e0e` (The Void)
- **Primary (Technical Orange):** `#ffb693` (Base) | `#7a3000` (Container)
- **Surface Hierarchy:** 
  - `surface-container-lowest`: `#000000`
  - `surface-container-low`: `#131313`
  - `surface-container-highest`: `#252626`
 
### The "No-Line" Rule
**Explicit Instruction:** You are prohibited from using 1px solid borders to define sections. In a high-precision environment, lines create visual noise. Boundaries must be defined solely through background color shifts. For example, a transcription editor (`surface-container-low`) should sit directly against the main background (`surface`) without a stroke. Use the `surface-container` tiers to indicate nesting and importance.
 
### The "Glass & Gradient" Rule
To prevent the UI from feeling "flat" or "cheap," use Glassmorphism for floating overlays (e.g., command palettes or tooltips). Utilize `surface-container-highest` at 70% opacity with a `20px` backdrop blur. 
**Signature Texture:** For primary CTAs or active recording states, apply a subtle linear gradient from `primary` (#ffb693)
 
---
 
## 3. Typography: Technical Authority
We employ a dual-typeface system to distinguish between **Interface** and **Data**.
 
### The Font Pairing
- **Interface:** `Geist`. Use this for headings, navigation, buttons, body copy, chips, and settings UI.
- **Data:** `Geist Mono`. Use this for eyebrows, transcript labels (`in:` / `out:`), timestamps, footer meta, paths, hotkeys, and compact technical values.
 
### Typography Scale
- **display-lg (Geist | `clamp(40px, 7vw, 84px)`):** Hero h1. Use `font-light`, `tracking-tight`, and `leading-none`. Use `font-medium` only for emphasized h1 words.
- **headline-lg (Geist | 2.25rem–3rem):** h2 section headings. Use `font-light`, `tracking-tight`, and `leading-[1.07]`.
- **subtitle (Geist | 17px):** Hero subtitles and high-level descriptions. Use relaxed leading.
- **body-md/body-sm (Geist | 13–14px):** Standard UI copy and descriptions. Default to `font-light` and relaxed leading.
- **label-xs/label-sm (Geist Mono | 10–11px):** Eyebrows, transcript labels, timestamps, and footer meta. Use `tracking-wide` or `tracking-widest`.

### Weight Rules
- **300 / `font-light`:** Default everywhere.
- **400 / `font-normal`:** Navigation, chips, logo, buttons, step headings, and compact labels.
- **500 / `font-medium`:** Strong words inside h1 only.

### Theme Modes
The app supports `system`, `dark`, and `light` theme modes from Settings. Components must use semantic tokens (`primary`, `secondary`, `active`, `normal`, `transparent`, `surface-*`, `on-surface`) rather than hardcoded colors so the mode switch can resolve centrally.

### Color Roles
Use a 60-30-10 hierarchy:
- **60% foundation:** `void`, `surface`, and `surface-container-*`.
- **30% support:** `secondary`, neutral panels, chips, and grouped controls.
- **10% emphasis:** `primary` for main actions and `active` for selected/current state. Keep `error` separate from accent.
 
---
 
## 4. Elevation & Depth: Tonal Layering
Traditional drop shadows are forbidden. They feel "web-like" and soft. Instead, we use **Tonal Layering**.
 
- **The Layering Principle:** Depth is achieved by stacking. A "floating" module is not "above" the UI; it is a higher-tier surface. Place a `surface-container-highest` card on top of a `surface-container-low` background.
- **Ambient Shadows:** If a floating element (like a context menu) requires separation, use an extra-diffused shadow: `0px 24px 48px rgba(0, 0, 0, 0.5)`. The shadow must feel like ambient light occlusion, not a "glow."
- **The "Ghost Border" Fallback:** If a container needs an edge for accessibility, use the `outline-variant` token at 15% opacity. It should be barely perceptible—a "whisper" of an edge.
- **Strict Geometry:** Corner radius is fixed at `4px` (`DEFAULT`). For smaller interior components (like checkboxes), use `2px` (`sm`). Never use `full` or `lg` radii.
 
---
 
## 5. Components: Modular Units
 
### Buttons
- **Primary:** Main action / brand color, `on-primary` text.
- **Secondary:** Supporting action/chip color, lower emphasis than primary.
- **Normal:** Default neutral control using surface/outline tokens.
- **Transparent:** Ghost control with no fill and a surface hover.
- **Active:** Selected/current state color, distinct from primary when the UI needs a separate state cue.
- **Destructive:** Error role only; do not reuse accent colors for destructive states.
 
### Technical Input Fields
- **Styling:** Use `surface-container-lowest` as the field background. 
- **States:** On focus, do not use a "glow." Change the background to `surface-container-high` and add a 1px `primary` (Orange) bottom-border only. This mimics professional audio rack hardware.
 
### Transcription Cards & Lists
- **No Dividers:** Forbid the use of horizontal lines between list items. Use vertical white space (16px/24px) or a subtle shift between `surface-container-low` and `surface-container-lowest` to separate entries.
- **Waveform Display:** Use `active` (#ff9e65) for inactive audio segments and `primary` (#ffb693) for active segments.
 
### Status Chips
- **Action Chips:** 4px radius. Background: `surface-container-highest`. Text: `label-md`.
- **Alert/Error:** Use `error_container` (#7e2b17) with `on_error_container` text. Keep it "burnt" and industrial, not bright "emergency" red.
 
---
 
## 6. Do’s and Don'ts
 
### Do
- **Do** embrace "Void Space." In an industrial UI, empty space is a sign of organization, not missing content.
- **Do** align everything to a strict 4px grid. If a label is 1px off, the "precision" illusion breaks.
- **Do** use monospaced numerals (available in Space Grotesk) for all timestamps and technical values.
 
### Don't
- **Don't** use icons with rounded caps. Use "sharp" or "square" icon sets to match the 4px radius theme.
- **Don't** introduce decorative blues, purples, or generic SaaS colors. `active` is the only separate state cue and must stay tokenized.
- **Don't** use center-alignment for headers. Everything should be left-aligned to mimic a technical log or ledger.
- **Don't** use standard 1px dividers. If you feel the need for a line, try a 4px background color shift instead.
- **Don't** Use disabled button workflows, they suck.