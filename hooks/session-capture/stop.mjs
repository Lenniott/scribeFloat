#!/usr/bin/env node
/** Stop hook — evaluate thresholds and prompt once per session. */
import {
  buildMessage,
  extractSuggestions,
  findStaleExplorations,
  formatStopOutput,
  getSessionId,
  loadState,
  platformFromInput,
  readStdinJson,
  saveState,
  shouldFire,
} from "./lib.mjs";

const input = readStdinJson();
const sessionId = getSessionId(input);
const state = loadState(sessionId);

if (state.fired) {
  process.exit(0);
}

// Claude: avoid re-blocking when already continuing from a prior stop hook.
if (input.stop_hook_active === true) {
  process.exit(0);
}

if (!shouldFire(state)) {
  process.exit(0);
}

state.fired = true;
saveState(sessionId, state);

const suggestions = extractSuggestions(input);
const stale = findStaleExplorations();
const message = buildMessage(suggestions, stale);
const platform = platformFromInput(input);

process.stdout.write(JSON.stringify(formatStopOutput(platform, message)));
