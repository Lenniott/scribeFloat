#!/usr/bin/env node
/**
 * Design-system lint check.
 *
 * Scans src/ .svelte files for violations of scribefloat token rules.
 * Run with:  node scripts/check-design-system.mjs
 * Exit 0 = all clear. Exit 1 = violations found.
 *
 * Add to package.json scripts: "check:ds": "node scripts/check-design-system.mjs"
 */

import { readFileSync, readdirSync, statSync } from "fs";
import { join, relative, extname } from "path";

const ROOT = new URL("../src", import.meta.url).pathname;
const PASS = "\x1b[32m✓\x1b[0m";
const FAIL = "\x1b[31m✗\x1b[0m";
const WARN = "\x1b[33m!\x1b[0m";

// ─── Rule definitions ────────────────────────────────────────────────────────
// Each rule: { id, description, pattern (global regex), fix, severity: "error"|"warn" }

const RULES = [
  // ── Undefined legacy tokens from old on-surface-*/surface-low/void naming ──
  {
    id: "no-surface-low",
    description: "surface-low is not a defined token — use card or rim",
    pattern: /\bsurface-low\b/g,
    fix: "border-*-surface-low → border-card or border-rim; var(--color-surface-low) → var(--sf-card)",
    severity: "error",
  },
  {
    id: "no-on-surface-dim",
    description: "on-surface-dim is not a defined token — use fg-muted",
    pattern: /\bon-surface-dim\b/g,
    fix: "bg-on-surface-dim → bg-fg-muted; decoration-on-surface-dim → decoration-fg-muted",
    severity: "error",
  },
  {
    id: "no-on-surface",
    description: "on-surface is not a defined token — use fg or fg-dim",
    // Avoid matching on-brand, on-active, on-destructive, on-warning, on-success
    pattern: /\bon-surface(?!-dim|-brand|-active|-destructive|-warning|-success)\b/g,
    fix: "bg-on-surface → bg-fg; bg-on-surface/40 → bg-fg/40 or bg-fg-muted",
    severity: "error",
  },
  {
    id: "no-text-void",
    description: "void is not a defined token — use on-brand",
    pattern: /\btext-void\b/g,
    fix: "text-void → text-on-brand",
    severity: "error",
  },

  // ── Radius: only rounded-sm (2px), rounded-md (4px), rounded-full ──────────
  {
    id: "no-rounded-lg",
    description: "rounded-lg violates the three-radius rule (sm / md / full only)",
    pattern: /\brounded-lg\b/g,
    fix: "rounded-lg → rounded-md",
    severity: "error",
  },
  {
    id: "no-rounded-xl",
    description: "rounded-xl violates the three-radius rule (sm / md / full only)",
    pattern: /\brounded-xl\b/g,
    fix: "rounded-xl → rounded-md",
    severity: "error",
  },
  {
    id: "no-rounded-2xl",
    description: "rounded-2xl violates the three-radius rule (sm / md / full only)",
    pattern: /\brounded-2xl\b/g,
    fix: "rounded-2xl → rounded-md",
    severity: "error",
  },
  {
    id: "no-rounded-3xl",
    description: "rounded-3xl violates the three-radius rule (sm / md / full only)",
    pattern: /\brounded-3xl\b/g,
    fix: "rounded-3xl → rounded-md",
    severity: "error",
  },

  // ── Shadow: only shadow-ambient allowed (PanelShell only) ──────────────────
  {
    id: "no-shadow-lg",
    description: "shadow-lg is not a design-system shadow — use shadow-ambient (PanelShell only) or remove",
    pattern: /\bshadow-lg\b/g,
    fix: "shadow-lg → shadow-ambient on PanelShell, or remove on everything else",
    severity: "error",
  },
  {
    id: "no-shadow-md",
    description: "shadow-md is not a design-system shadow",
    pattern: /\bshadow-md\b/g,
    fix: "shadow-md → remove (elevation via surface lightness, not shadow)",
    severity: "error",
  },
  {
    id: "no-shadow-sm",
    description: "shadow-sm is not a design-system shadow",
    pattern: /\bshadow-sm\b/g,
    fix: "shadow-sm → remove",
    severity: "error",
  },
  {
    id: "no-shadow-xl",
    description: "shadow-xl is not a design-system shadow",
    pattern: /\bshadow-xl\b/g,
    fix: "shadow-xl → shadow-ambient on PanelShell, or remove",
    severity: "error",
  },

  // ── Typography: weights 400 and 500 only (no bold) ─────────────────────────
  {
    id: "no-font-bold",
    description: "font-bold violates the no-bold rule — hierarchy via size/case/opacity only",
    pattern: /\bfont-bold\b/g,
    fix: "Remove font-bold; use font-medium (500) or size/opacity for hierarchy",
    severity: "error",
  },
  {
    id: "no-font-semibold",
    description: "font-semibold violates the no-bold rule",
    pattern: /\bfont-semibold\b/g,
    fix: "font-semibold → font-medium at most",
    severity: "error",
  },
  {
    id: "no-font-extrabold",
    description: "font-extrabold violates the no-bold rule",
    pattern: /\bfont-extrabold\b/g,
    fix: "font-extrabold → font-medium at most",
    severity: "error",
  },

  // ── Undefined CSS variable references ──────────────────────────────────────
  {
    id: "no-color-surface-low-var",
    description: "var(--color-surface-low) references an undefined CSS variable",
    pattern: /var\(--color-surface-low\)/g,
    fix: "var(--color-surface-low) → var(--sf-card)",
    severity: "error",
  },

  // ── Hardcoded hex colors (warn — may be intentional in rare edge cases) ─────
  {
    id: "no-hardcoded-hex",
    description: "Hardcoded hex color — should use --sf-* tokens",
    pattern: /(?:class|style)=[^>]*#[0-9a-fA-F]{3,8}\b/g,
    fix: "Replace with the nearest --sf-* token or Tailwind utility",
    severity: "warn",
  },
];

// ─── File walker ─────────────────────────────────────────────────────────────

function* walkSvelte(dir) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory() && !entry.startsWith(".") && entry !== "node_modules") {
      yield* walkSvelte(full);
    } else if (stat.isFile() && extname(full) === ".svelte") {
      yield full;
    }
  }
}

// ─── Run checks ──────────────────────────────────────────────────────────────

let totalErrors = 0;
let totalWarnings = 0;
const findings = [];

for (const file of walkSvelte(ROOT)) {
  const src = readFileSync(file, "utf8");
  const lines = src.split("\n");
  const rel = relative(process.cwd(), file);

  for (const rule of RULES) {
    rule.pattern.lastIndex = 0;
    let match;
    while ((match = rule.pattern.exec(src)) !== null) {
      const before = src.slice(0, match.index);
      const lineNum = before.split("\n").length;
      const lineText = lines[lineNum - 1]?.trim() ?? "";

      findings.push({ rule, file: rel, lineNum, lineText, match: match[0] });

      if (rule.severity === "error") totalErrors++;
      else totalWarnings++;

      if (match.index === rule.pattern.lastIndex) rule.pattern.lastIndex++;
    }
  }
}

// ─── Output ──────────────────────────────────────────────────────────────────

if (findings.length === 0) {
  console.log(`${PASS} Design system check passed — no violations found.`);
  process.exit(0);
}

const byFile = new Map();
for (const f of findings) {
  if (!byFile.has(f.file)) byFile.set(f.file, []);
  byFile.get(f.file).push(f);
}

for (const [file, ff] of byFile) {
  console.log(`\n${file}`);
  for (const { rule, lineNum, lineText, match } of ff) {
    const icon = rule.severity === "error" ? FAIL : WARN;
    console.log(`  ${icon} [${rule.id}] line ${lineNum}: ${rule.description}`);
    console.log(`       Found: "${match}" in: ${lineText.slice(0, 120)}`);
    console.log(`       Fix:   ${rule.fix}`);
  }
}

console.log(
  `\n${totalErrors > 0 ? FAIL : PASS} ${totalErrors} error(s), ${totalWarnings} warning(s) across ${byFile.size} file(s).`
);

if (totalErrors > 0) process.exit(1);
