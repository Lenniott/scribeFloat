#!/usr/bin/env bash
# Sync skills/ to all agent tool locations, and keep AGENTS.md ↔ CLAUDE.md in sync.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SKILLS_SRC="$REPO_ROOT/skills"
CURSOR_DST="$REPO_ROOT/.cursor/skills"
CLAUDE_DST="$REPO_ROOT/.claude/commands"
AGENTS_FILE="$REPO_ROOT/AGENTS.md"
CLAUDE_FILE="$REPO_ROOT/CLAUDE.md"

changed=0

# 1. Sync skills/ → .cursor/skills/
mkdir -p "$CURSOR_DST"
rsync -a --update --exclude "build.sh" "$SKILLS_SRC/" "$CURSOR_DST/"
echo "✓ skills → .cursor/skills/"

# 2. Sync skills/ → .claude/commands/
mkdir -p "$CLAUDE_DST"
rsync -a --update --exclude "build.sh" "$SKILLS_SRC/" "$CLAUDE_DST/"
echo "✓ skills → .claude/commands/"

# 3. Sync AGENTS.md ↔ CLAUDE.md by mtime (newer wins)
if [[ -f "$AGENTS_FILE" && -f "$CLAUDE_FILE" ]]; then
  if [[ "$AGENTS_FILE" -nt "$CLAUDE_FILE" ]]; then
    cp "$AGENTS_FILE" "$CLAUDE_FILE"
    echo "✓ AGENTS.md → CLAUDE.md (AGENTS.md was newer)"
    changed=1
  elif [[ "$CLAUDE_FILE" -nt "$AGENTS_FILE" ]]; then
    cp "$CLAUDE_FILE" "$AGENTS_FILE"
    echo "✓ CLAUDE.md → AGENTS.md (CLAUDE.md was newer)"
    changed=1
  else
    echo "✓ AGENTS.md ↔ CLAUDE.md (in sync)"
  fi
elif [[ -f "$AGENTS_FILE" && ! -f "$CLAUDE_FILE" ]]; then
  cp "$AGENTS_FILE" "$CLAUDE_FILE"
  echo "✓ AGENTS.md → CLAUDE.md (created)"
elif [[ -f "$CLAUDE_FILE" && ! -f "$AGENTS_FILE" ]]; then
  cp "$CLAUDE_FILE" "$AGENTS_FILE"
  echo "✓ CLAUDE.md → AGENTS.md (created)"
fi

echo "build.sh done"
