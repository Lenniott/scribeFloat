#!/usr/bin/env node
/**
 * Cursor hooks: UI enforcement for frontend files.
 *
 * - preToolUse (Write|StrReplace): deny bad typography before write; agent_message = cheat sheet
 * - afterFileEdit / postToolUse: run check:ds (postToolUse additional_context is best-effort)
 */

import { execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import {
  ROOT,
  CHEAT_SHEET,
  denyMessage,
  extractFilePath,
  extractWriteContent,
  isFrontendPath,
  relPathSync,
  scanDeny,
} from "./ui-enforcement-lib.mjs";

function readStdin() {
  try {
    return JSON.parse(readFileSync(0, "utf8"));
  } catch {
    return {};
  }
}

function runCheckDs() {
  try {
    execSync("npm run check:ds", { cwd: ROOT, stdio: "pipe", encoding: "utf8" });
    return true;
  } catch (err) {
    const out = [err.stdout, err.stderr].filter(Boolean).join("\n").trim();
    console.error(`[ui-enforcement] check:ds failed:\n${out}`);
    return false;
  }
}

function handlePreToolUse(input) {
  const tool = input.tool_name ?? "";
  const filePath = extractFilePath(input);
  if (!filePath || !isFrontendPath(filePath)) {
    process.stdout.write(JSON.stringify({ permission: "allow" }));
    return;
  }

  const chunk = extractWriteContent(tool, input.tool_input);
  if (!chunk) {
    process.stdout.write(JSON.stringify({ permission: "allow" }));
    return;
  }

  const fileRel = relPathSync(filePath);
  const hits = scanDeny(chunk, fileRel);
  if (hits.length === 0) {
    process.stdout.write(JSON.stringify({ permission: "allow" }));
    return;
  }

  process.stdout.write(
    JSON.stringify({
      permission: "deny",
      agent_message: denyMessage(fileRel, hits),
    }),
  );
}

function handleAfterEdit(input) {
  const filePath = input.file_path;
  if (!filePath || !isFrontendPath(filePath)) {
    process.stdout.write("{}");
    return;
  }

  const fileRel = relPathSync(filePath);
  try {
    const content = readFileSync(resolve(ROOT, fileRel), "utf8");
    const hits = scanDeny(content, fileRel);
    if (hits.length) {
      console.error(
        `[ui-enforcement] ${fileRel}: still has ${hits.map((h) => h.id).join(", ")} — migrate to sf-*`,
      );
    }
  } catch (err) {
    console.error(`[ui-enforcement] could not read ${fileRel}: ${err.message}`);
  }

  runCheckDs();
  process.stdout.write("{}");
}

function handlePostToolUse(input) {
  const filePath = extractFilePath(input);
  if (!filePath || !isFrontendPath(filePath)) {
    process.stdout.write("{}");
    return;
  }

  const fileRel = relPathSync(filePath);
  const ok = runCheckDs();
  const reminder = ok
    ? `UI OK: ${fileRel}. ${CHEAT_SHEET.split("\n")[0]}`
    : `check:ds failed after editing ${fileRel}. Fix violations before continuing.`;

  process.stdout.write(JSON.stringify({ additional_context: reminder }));
}

function main() {
  const input = readStdin();
  const event = input.hook_event_name ?? "";

  switch (event) {
    case "preToolUse":
      handlePreToolUse(input);
      return;
    case "afterFileEdit":
      handleAfterEdit(input);
      return;
    case "postToolUse":
      handlePostToolUse(input);
      return;
    default:
      break;
  }

  // Shape-based fallback when hook_event_name is omitted
  if (input.file_path && input.edits) {
    handleAfterEdit(input);
  } else if (input.tool_output !== undefined) {
    handlePostToolUse(input);
  } else if (input.tool_name) {
    handlePreToolUse(input);
  } else {
    process.stdout.write("{}");
  }
}

main();
