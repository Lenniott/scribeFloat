/** Shared UI enforcement patterns — used by Cursor hooks. */

import { relative } from "node:path";

export const ROOT = new URL("../..", import.meta.url).pathname;

export const FRONTEND_RE = /(?:^|\/)src\/.*\.(?:svelte|css)$/;
export const EXEMPT_RE = /routes\/design-system\//;

/** Typography — block on write to .svelte and .css */
export const TYPOGRAPHY_DENY_PATTERNS = [
  { id: "text-base", re: /\btext-base\b/, fix: "sf-body-md" },
  { id: "text-sm", re: /\btext-sm\b/, fix: "sf-label-sm or sf-body-md" },
  { id: "text-headline-lg", re: /\btext-headline-lg\b/, fix: "sf-headline-sm" },
  { id: "text-body-lg", re: /\btext-body-lg\b/, fix: "sf-body-md" },
  { id: "tracking-heading", re: /\btracking-heading\b/, fix: "tracking-stamped" },
  {
    id: "inline-field-label",
    re: /text-label-sm\s+font-normal\s+tracking-stamped[^"']*uppercase/,
    fix: "sf-field-label",
  },
  {
    id: "inline-label-recipe",
    re: /text-label-sm\s+font-normal\s+tracking-stamped\s+text-fg-dim\s+uppercase/,
    fix: "sf-field-label",
  },
];

/** Color Option A — .svelte only (app.css is token source) */
export const COLOR_DENY_PATTERNS = [
  {
    id: "text-fg-opacity",
    re: /\btext-fg\/\d+\b/,
    fix: "text-fg, text-fg-dim, or text-fg-muted",
  },
  {
    id: "bg-fg-opacity",
    re: /\bbg-fg\/\d+\b/,
    fix: "bg-fg-muted or a surface token",
  },
  { id: "bg-black-scrim", re: /\bbg-black\/\d+\b/, fix: "sf-scrim or bg-overlay" },
];

export const COLOR_CHEATSHEET = `ScribeFloat color (Option A — semantic fg only):

| Level | Class |
| Primary | text-fg |
| Secondary / captions | text-fg-dim |
| Muted / empty / disabled | text-fg-muted |

Modal scrim: sf-scrim or bg-overlay (not bg-black/50).
State: brand=CTA · active=selected · destructive=errors · warning=caution · success=confirmed.
Banned in .svelte: text-fg/40–text-fg/90, bg-black/50.`;

/** Inlined in rule + preToolUse deny — agents need not open other files */
export const CHEAT_SHEET = `ScribeFloat typography — use sf-* from src/app.css (+ semantic color):

| Element | Class |
|---------|-------|
| Page title h1/h2 | sf-headline-sm text-fg |
| Section header | sf-section-label text-fg-dim |
| Body / description | sf-body-md text-fg or text-fg-dim |
| Emphasized body | sf-body-md-strong text-fg |
| Form label / legend | sf-field-label |
| Tab / button / chip text | sf-label-md text-fg |
| Column header / badge | sf-label-sm text-fg-dim |
| Timestamp / duration | sf-meta-sm text-fg-dim |
| Hero (onboarding) | sf-display-lg text-fg |

${COLOR_CHEATSHEET}`;

export function relPathSync(absPath, root = ROOT) {
  if (!absPath) return "";
  try {
    return relative(root, absPath).replace(/\\/g, "/");
  } catch {
    return String(absPath);
  }
}

export function isFrontendPath(filePath, root = ROOT) {
  const rel = relPathSync(filePath, root);
  return FRONTEND_RE.test(rel) && !EXEMPT_RE.test(rel);
}

export function isSveltePath(fileRel) {
  return fileRel.endsWith(".svelte");
}

/** Token source — hooks must not block foundation edits */
export function isTokenSourcePath(fileRel) {
  return fileRel === "src/app.css";
}

export function scanDeny(content, fileRel = "") {
  if (isTokenSourcePath(fileRel)) return [];

  const patterns = [...TYPOGRAPHY_DENY_PATTERNS];
  if (isSveltePath(fileRel)) patterns.push(...COLOR_DENY_PATTERNS);

  const hits = [];
  for (const p of patterns) {
    p.re.lastIndex = 0;
    if (p.re.test(content)) hits.push(p);
  }
  return hits;
}

export function extractWriteContent(tool, toolInput) {
  const ti = toolInput ?? {};
  if (/^write$/i.test(tool)) return ti.contents ?? ti.content ?? "";
  if (/^strreplace$/i.test(tool)) return ti.new_string ?? "";
  return "";
}

export function extractFilePath(input) {
  if (input.file_path) return input.file_path;

  const tool = input.tool_name ?? "";
  if (!/^(Write|StrReplace)$/i.test(tool)) return null;

  const ti = input.tool_input ?? {};
  return ti.path ?? ti.file_path ?? ti.target_file ?? null;
}

export function denyMessage(fileRel, hits) {
  const violations = hits.map((h) => `${h.id} → ${h.fix}`).join("; ");
  return `${CHEAT_SHEET}\n\nBlocked write to ${fileRel}: ${violations}.\nRedo this edit using tokens above.`;
}
