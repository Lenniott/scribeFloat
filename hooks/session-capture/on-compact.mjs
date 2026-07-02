#!/usr/bin/env node
/** Record context pressure before compaction (≥60% or any auto-compact). */
import { CONTEXT_THRESHOLD, getSessionId, loadState, readStdinJson, saveState } from "./lib.mjs";

const input = readStdinJson();
const sessionId = getSessionId(input);
const state = loadState(sessionId);

const percent = Number(input.context_usage_percent ?? 0);
if (percent >= CONTEXT_THRESHOLD) {
  state.contextPercent = percent;
  state.contextPressure = true;
} else if (input.trigger === "auto" || input.hook_event_name === "PreCompact") {
  // Claude PreCompact has no percent — compaction implies high context usage.
  state.contextPressure = true;
  state.contextPercent = Math.max(state.contextPercent ?? 0, CONTEXT_THRESHOLD);
}

saveState(sessionId, state);
