#!/usr/bin/env node
/**
 * Shared session-capture policy — state, thresholds, message building.
 * Used by Claude Code and Cursor hook entrypoints.
 */

import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { execSync } from "node:child_process";

export const TURN_THRESHOLD = 15;
export const CONTEXT_THRESHOLD = 60;
export const STALE_DAYS = 30;

const ROOT = process.env.CURSOR_PROJECT_DIR ?? process.env.CLAUDE_PROJECT_DIR ?? process.cwd();

export function getSessionId(input) {
  const raw = input.session_id ?? input.conversation_id ?? "unknown";
  return String(raw).slice(0, 16);
}

export function stateDir(sessionId) {
  return `/tmp/session-capture-${sessionId}`;
}

function statePath(sessionId) {
  return join(stateDir(sessionId), "state.json");
}

export function loadState(sessionId) {
  const dir = stateDir(sessionId);
  mkdirSync(dir, { recursive: true });
  const path = statePath(sessionId);
  if (!existsSync(path)) {
    return { turns: 0, contextPressure: false, contextPercent: 0, fired: false };
  }
  try {
    return { turns: 0, contextPressure: false, contextPercent: 0, fired: false, ...JSON.parse(readFileSync(path, "utf8")) };
  } catch {
    return { turns: 0, contextPressure: false, contextPercent: 0, fired: false };
  }
}

export function saveState(sessionId, state) {
  mkdirSync(stateDir(sessionId), { recursive: true });
  writeFileSync(statePath(sessionId), JSON.stringify(state, null, 0));
}

export function isDocPath(filePath) {
  if (!filePath) return false;
  const norm = filePath.replace(/\\/g, "/");
  return (
    norm.includes("docs/adr/") ||
    norm.includes(".scratch/") ||
    norm.includes("docs/agents/")
  );
}

export function findStaleExplorations() {
  const stale = [];
  const now = Date.now();
  const dirs = ["docs/explorations/active", "docs/explorations"];

  for (const dir of dirs) {
    const abs = join(ROOT, dir);
    if (!existsSync(abs)) continue;
    let files;
    try {
      files = readdirSync(abs).filter((f) => f.endsWith(".md") && f !== "README.md");
    } catch {
      continue;
    }
    for (const name of files) {
      const full = join(abs, name);
      if (dir === "docs/explorations" && (full.includes("/active/") || full.includes("/captured/") || full.includes("/stale/"))) {
        continue;
      }

      // active/ folder = not captured by definition; root files use frontmatter.
      if (dir !== "docs/explorations/active") {
        try {
          const head = readFileSync(full, "utf8").slice(0, 400);
          const m = head.match(/^status:\s*(\S+)/m);
          if (m?.[1] === "captured") continue;
        } catch {
          continue;
        }
      }

      const ageDays = Math.floor((now - statSync(full).mtimeMs) / 86_400_000);
      if (ageDays > STALE_DAYS) {
        stale.push({ name: dir === "docs/explorations/active" ? `active/${name}` : name, ageDays });
      }
    }
  }

  return stale.slice(0, 8);
}

const SUGGESTION_PATTERNS = [
  /\b(decid(e|ed|ing)|chose|picked)\b.{10,120}/gi,
  /\binstead of\b.{10,120}/gi,
  /\btrade-?off\b.{10,120}/gi,
  /\b(we|I) (will|should|'ll) (use|go with|prefer)\b.{10,120}/gi,
  /\b(ADR|architecture|pattern|non-obvious)\b.{10,120}/gi,
  /\b(refactor|rename|deprecate|move .{5,40} to)\b.{10,120}/gi,
];

function cleanSnippet(text) {
  return text.replace(/\s+/g, " ").trim().slice(0, 140);
}

function readTranscriptLines(transcriptPath, maxLines = 80) {
  if (!transcriptPath || !existsSync(transcriptPath)) return [];
  try {
    const lines = readFileSync(transcriptPath, "utf8").trim().split("\n").slice(-maxLines);
    const texts = [];
    for (const line of lines) {
      try {
        const obj = JSON.parse(line);
        const parts = [];
        if (typeof obj.message?.content === "string") parts.push(obj.message.content);
        if (Array.isArray(obj.message?.content)) {
          for (const block of obj.message.content) {
            if (typeof block?.text === "string") parts.push(block.text);
          }
        }
        if (typeof obj.content === "string") parts.push(obj.content);
        if (typeof obj.text === "string") parts.push(obj.text);
        if (typeof obj.last_assistant_message === "string") parts.push(obj.last_assistant_message);
        texts.push(...parts);
      } catch {
        if (line.trim()) texts.push(line);
      }
    }
    return texts;
  } catch {
    return [];
  }
}

export function extractSuggestions(input) {
  const transcriptPath =
    input.transcript_path ?? process.env.CURSOR_TRANSCRIPT_PATH ?? "";
  const texts = readTranscriptLines(transcriptPath);

  if (input.last_assistant_message) texts.push(input.last_assistant_message);
  if (input.prompt) texts.push(input.prompt);

  const joined = texts.join("\n");
  const found = new Set();

  for (const pattern of SUGGESTION_PATTERNS) {
    pattern.lastIndex = 0;
    let match;
    while ((match = pattern.exec(joined)) !== null && found.size < 5) {
      const snippet = cleanSnippet(match[0]);
      if (snippet.length > 20) found.add(snippet);
    }
  }

  if (found.size < 3) {
    try {
      const changed = execSync("git diff --name-only HEAD 2>/dev/null || git diff --name-only", {
        cwd: ROOT,
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
      })
        .trim()
        .split("\n")
        .filter(Boolean)
        .filter(
          (p) =>
            !p.startsWith("docs/adr/") &&
            !p.startsWith(".scratch/") &&
            !p.startsWith("docs/agents/")
        )
        .slice(0, 5);
      for (const p of changed) {
        found.add(`Code/docs changed without a matching story or ADR: ${p}`);
        if (found.size >= 5) break;
      }
    } catch {
      /* no git */
    }
  }

  return [...found].slice(0, 5);
}

export function shouldFire(state) {
  if (state.fired) return false;
  const turnHit = state.turns >= TURN_THRESHOLD;
  const contextHit =
    state.contextPressure || (state.contextPercent ?? 0) >= CONTEXT_THRESHOLD;
  return turnHit || contextHit;
}

export function buildMessage(suggestions, stale) {
  const lines = [
    "## Session capture",
    "",
    "Are there any stories or decisions from this session that need capturing?",
    "",
    "**Might be worth capturing:**",
  ];

  if (suggestions.length === 0) {
    lines.push("- Review the session for architectural choices, tradeoffs, or non-obvious implementation decisions.");
  } else {
    for (const s of suggestions) lines.push(`- ${s}`);
  }

  lines.push("");
  if (stale.length > 0) {
    lines.push("**Stale explorations** (>30 days, not captured) — consider moving to `docs/explorations/stale/`:");
    for (const { name, ageDays } of stale) lines.push(`- ${name} (${ageDays}d)`);
    lines.push("");
  }

  lines.push(
    "Record binding decisions under `docs/adr/`; effort work under `.scratch/<effort>/`.",
    "Reply **nothing to capture** to dismiss this check for the rest of the session.",
  );

  return lines.join("\n");
}

export function platformFromInput(input) {
  if (process.env.CURSOR_VERSION) return "cursor";
  if (input.hook_event_name === "Stop" || input.hook_event_name === "stop") {
    return input.conversation_id ? "cursor" : "claude";
  }
  return input.conversation_id && !input.session_id ? "cursor" : "claude";
}

export function formatStopOutput(platform, message) {
  if (platform === "cursor") {
    return { followup_message: message };
  }
  return {
    hookSpecificOutput: {
      hookEventName: "Stop",
      additionalContext: message,
    },
  };
}

export function readStdinJson() {
  try {
    return JSON.parse(readFileSync(0, "utf8"));
  } catch {
    return {};
  }
}
