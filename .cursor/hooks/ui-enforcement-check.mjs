#!/usr/bin/env node
/**
 * Cursor hooks: UI enforcement for frontend files.
 *
 * - preToolUse: deny if write chunk OR resulting full file has violations
 * - postToolUse: inject additional_context when edited file still violates (reliable feedback)
 * - afterFileEdit: full-file scan + check:ds (stderr log; postToolUse carries agent context)
 */

import { execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import {
  ROOT,
  auditFileOnDisk,
  denyMessage,
  extractFilePath,
  extractWriteContent,
  followUpMessage,
  isFrontendPath,
  relPathSync,
  resultingFileContent,
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
    console.error(`[ui-enforcement] check:ds errors:\n${out}`);
    return false;
  }
}

function collectHits(tool, toolInput, fileRel) {
  const chunk = extractWriteContent(tool, toolInput);
  const chunkHits = chunk ? scanDeny(chunk, fileRel) : [];

  const resulting = resultingFileContent(tool, toolInput, fileRel);
  const fileHits = resulting ? scanDeny(resulting, fileRel) : [];

  const byId = new Map();
  for (const hit of [...chunkHits, ...fileHits]) {
    byId.set(hit.id, hit);
  }
  return [...byId.values()];
}

function handlePreToolUse(input) {
  const tool = input.tool_name ?? "";
  const filePath = extractFilePath(input);
  if (!filePath || !isFrontendPath(filePath)) {
    process.stdout.write(JSON.stringify({ permission: "allow" }));
    return;
  }

  const fileRel = relPathSync(filePath);
  const hits = collectHits(tool, input.tool_input, fileRel);
  if (hits.length === 0) {
    process.stdout.write(JSON.stringify({ permission: "allow" }));
    return;
  }

  const scope =
    extractWriteContent(tool, input.tool_input) && resultingFileContent(tool, input.tool_input, fileRel)
      ? "Blocked write — chunk or resulting file"
      : "Blocked write";

  process.stdout.write(
    JSON.stringify({
      permission: "deny",
      agent_message: denyMessage(fileRel, hits, scope),
    }),
  );
}

function handleAfterEdit(input) {
  const filePath = input.file_path ?? extractFilePath(input);
  if (!filePath || !isFrontendPath(filePath)) {
    process.stdout.write("{}");
    return;
  }

  const fileRel = relPathSync(filePath);
  const hits = auditFileOnDisk(fileRel);
  if (hits.length) {
    console.error(
      `[ui-enforcement] ${fileRel}: ${hits.map((h) => h.id).join(", ")} — migrate to sf-*`,
    );
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
  const hits = auditFileOnDisk(fileRel);
  runCheckDs();

  if (hits.length === 0) {
    process.stdout.write("{}");
    return;
  }

  process.stdout.write(
    JSON.stringify({
      additional_context: followUpMessage(fileRel, hits),
    }),
  );
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
