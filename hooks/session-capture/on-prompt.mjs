#!/usr/bin/env node
/** Increment turn counter on each user prompt (silent). */
import { getSessionId, loadState, readStdinJson, saveState } from "./lib.mjs";

const input = readStdinJson();
const prompt = (input.prompt ?? "").trim().toLowerCase();

// Dismissal path — record session as handled.
if (/^nothing to capture\.?$/.test(prompt)) {
  const sessionId = getSessionId(input);
  const state = loadState(sessionId);
  state.fired = true;
  saveState(sessionId, state);
  process.exit(0);
}

const sessionId = getSessionId(input);
const state = loadState(sessionId);
if (!state.fired) {
  state.turns = (state.turns ?? 0) + 1;
  saveState(sessionId, state);
}
