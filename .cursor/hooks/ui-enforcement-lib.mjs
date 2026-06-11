/** Shared UI enforcement patterns — used by Cursor hooks. */

import { readFileSync } from "node:fs";
import { relative, resolve } from "node:path";

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
    id: "uppercase",
    re: /\buppercase\b/,
    fix: "Remove uppercase; sf-* label/header roles use capitalize",
  },
  {
    id: "font-mono-ui",
    re: /\bfont-mono\b/,
    fix: "sf-* roles use font-sans; mono only for instrument readouts",
  },
  {
    id: "inline-label-md-stamped",
    re: /\btext-label-md\b[^"']*tracking-stamped/,
    fix: "sf-label-md or sf-section-label or sf-headline-sm",
  },
  {
    id: "inline-label-sm-stamped",
    re: /\btext-label-sm\b[^"']*tracking-stamped/,
    fix: "sf-label-sm, sf-field-label, or sf-meta-sm",
  },
  {
    id: "inline-body-md",
    re: /\btext-body-md\b/,
    fix: "sf-body-md or sf-body-md-strong",
  },
  {
    id: "inline-field-label",
    re: /text-label-sm\s+font-normal\s+tracking-stamped/,
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
| Secondary / icons / chevrons (readable on card) | text-fg-dim |
| Recessed helper/empty copy only | text-fg-muted |

Never text-fg-muted on icons or expand controls — text-fg/50 on bg-card ≈ text-fg-dim, not muted.
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

Case: labels/headers → capitalize via sf-* (never uppercase). Body → sentence case.

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

export function readProjectFile(fileRel, root = ROOT) {
  return readFileSync(resolve(root, fileRel), "utf8");
}

/** Simulate StrReplace / Write merge for preToolUse full-file audit */
export function resultingFileContent(tool, toolInput, fileRel, root = ROOT) {
  const ti = toolInput ?? {};
  const chunk = extractWriteContent(tool, ti);
  if (!chunk && !/^strreplace$/i.test(tool ?? "")) return null;

  if (/^write$/i.test(tool)) return chunk;

  if (/^strreplace$/i.test(tool)) {
    const oldStr = ti.old_string ?? "";
    const newStr = ti.new_string ?? "";
    if (!oldStr) return chunk || null;
    try {
      const current = readProjectFile(fileRel, root);
      if (!current.includes(oldStr)) return chunk || null;
      if (ti.replace_all) return current.split(oldStr).join(newStr);
      const idx = current.indexOf(oldStr);
      return current.slice(0, idx) + newStr + current.slice(idx + oldStr.length);
    } catch {
      return chunk || null;
    }
  }

  return chunk || null;
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

export function denyMessage(fileRel, hits, scope = "Blocked write") {
  const violations = hits.map((h) => `${h.id} → ${h.fix}`).join("; ");
  return `${CHEAT_SHEET}\n\n${scope} to ${fileRel}: ${violations}.\nMigrate the whole file to sf-* + semantic color before saving.`;
}

export function followUpMessage(fileRel, hits) {
  const violations = hits.map((h) => `${h.id} → ${h.fix}`).join("; ");
  return `${CHEAT_SHEET}\n\nUI enforcement: ${fileRel} still violates ${violations}.\nFix all violations in this file before continuing other work.`;
}

export function auditFileOnDisk(fileRel, root = ROOT) {
  try {
    const content = readProjectFile(fileRel, root);
    return scanDeny(content, fileRel);
  } catch {
    return [];
  }
}
