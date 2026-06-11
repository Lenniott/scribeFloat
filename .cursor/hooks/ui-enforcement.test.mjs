#!/usr/bin/env node
/** Smoke tests for ui-enforcement hooks — run: node .cursor/hooks/ui-enforcement.test.mjs */

import { execSync } from "node:child_process";
import { writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import assert from "node:assert/strict";
import {
  scanDeny,
} from "./ui-enforcement-lib.mjs";

const hook = ".cursor/hooks/ui-enforcement-check.mjs";
const root = process.cwd();

function runHook(payload) {
  const out = execSync(`node ${hook}`, {
    input: JSON.stringify(payload),
    encoding: "utf8",
    cwd: root,
  });
  return JSON.parse(out || "{}");
}

let failed = 0;
function test(label, fn) {
  try {
    fn();
    console.log(`✓ ${label}`);
  } catch (e) {
    failed++;
    console.error(`✗ ${label}:`, e.message);
  }
}

test("scanDeny catches uppercase", () => {
  assert.ok(scanDeny('class="uppercase"', "src/lib/foo.svelte").some((h) => h.id === "uppercase"));
});

test("scanDeny catches text-fg/80 on svelte", () => {
  assert.ok(
    scanDeny('class="text-fg/80"', "src/lib/foo.svelte").some((h) => h.id === "text-fg-opacity"),
  );
});

test("preToolUse denies uppercase in write chunk", () => {
  const res = runHook({
    hook_event_name: "preToolUse",
    tool_name: "Write",
    tool_input: {
      path: join(root, "src/lib/components/Foo.svelte"),
      contents: '<p class="uppercase text-fg">x</p>',
    },
  });
  assert.equal(res.permission, "deny");
  assert.match(res.agent_message, /uppercase/);
});

test("preToolUse denies StrReplace when resulting file still has uppercase", () => {
  const rel = "src/lib/components/_hook_test.svelte";
  const abs = join(root, rel);
  writeFileSync(
    abs,
    `<div class="uppercase text-fg/80">A</div>\n<div class="sf-body-md text-fg">B</div>\n`,
  );
  try {
    const res = runHook({
      hook_event_name: "preToolUse",
      tool_name: "StrReplace",
      tool_input: {
        path: abs,
        old_string: '<div class="sf-body-md text-fg">B</div>',
        new_string: '<div class="sf-body-md text-fg">C</div>',
      },
    });
    assert.equal(res.permission, "deny");
    assert.match(res.agent_message, /uppercase|text-fg-opacity/);
  } finally {
    rmSync(abs, { force: true });
  }
});

test("preToolUse allows fully migrated write", () => {
  const res = runHook({
    hook_event_name: "preToolUse",
    tool_name: "Write",
    tool_input: {
      path: join(root, "src/lib/components/Foo.svelte"),
      contents: '<p class="sf-body-md text-fg">Hello</p>',
    },
  });
  assert.equal(res.permission, "allow");
});

test("postToolUse injects context when file on disk still violates", () => {
  const rel = "src/lib/components/_hook_test_post.svelte";
  const abs = join(root, rel);
  writeFileSync(abs, '<p class="uppercase text-fg/60">bad</p>');
  try {
    const res = runHook({
      hook_event_name: "postToolUse",
      tool_name: "Write",
      tool_input: { path: abs },
      tool_output: "ok",
    });
    assert.ok(res.additional_context);
    assert.match(res.additional_context, /uppercase/);
    assert.match(res.additional_context, /text-fg-opacity/);
  } finally {
    rmSync(abs, { force: true });
  }
});

test("postToolUse silent when migrated file clean", () => {
  const res = runHook({
    hook_event_name: "postToolUse",
    tool_name: "Write",
    tool_input: { path: join(root, "src/lib/components/Button.svelte") },
    tool_output: "ok",
  });
  assert.deepEqual(res, {});
});

if (failed) process.exit(1);
console.log(`\nAll ${7} hook tests passed`);
