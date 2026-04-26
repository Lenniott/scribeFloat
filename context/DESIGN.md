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
We employ a dual-typeface system to distinguish between **Data** and **Interface**.
 
### The Font Pairing
- **Technical Headers & Data:** `Space Grotesk`. Its idiosyncratic terminals and monospaced-leaning proportions convey engineering precision. Use this for all headings, timestamps, and raw transcript data.
- **UI Text:** `Inter`. A neutral, highly legible workhorse for buttons, labels, and system messages.
 
### Typography Scale
- **display-lg (Space Grotesk | 3.5rem):** Reserved for hero data points or start-state branding.
- **headline-sm (Space Grotesk | 1.5rem):** Section headers. Use all-caps with 0.05em tracking for an industrial "stamped" look.
- **body-md (Inter | 0.875rem):** Standard UI text. Use a `1.5` line-height to maintain "breathing room" amidst high data density.
- **label-sm (Inter | 0.6875rem):** For metadata (bitrate, file size, technical specs). Always in semi-bold.
 
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
- **Primary:** Technical Orange gradient background, `on-primary` text (Space Grotesk, Semi-bold). 4px radius.
- **Secondary:** `surface-container-highest` background, `on-surface` text. No border.
- **active:** Ghost style. No background. `primary` text. Underline only on hover.
 
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
- **Don't** use blues, purples, or "SaaS Blue." The palette is strictly monochromatic + Technical Orange.
- **Don't** use center-alignment for headers. Everything should be left-aligned to mimic a technical log or ledger.
- **Don't** use standard 1px dividers. If you feel the need for a line, try a 4px background color shift instead.
- **Don't** Use disabled button workflows, they suck.