#!/usr/bin/env node
/** Reset turn counter when backlog or ADR files are written. */
import { getSessionId, isDocPath, loadState, readStdinJson, saveState } from "./lib.mjs";

const input = readStdinJson();
const toolInput = input.tool_input ?? {};
const path =
  toolInput.file_path ?? toolInput.path ?? toolInput.target_file ?? "";

if (!isDocPath(path)) {
  process.exit(0);
}

const sessionId = getSessionId(input);
const state = loadState(sessionId);
state.turns = 0;
state.contextPressure = false;
state.contextPercent = 0;
saveState(sessionId, state);
