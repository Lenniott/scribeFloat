#!/usr/bin/env bash
# Sync skills/ to all agent tool locations.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SKILLS_SRC="$REPO_ROOT/skills"
CURSOR_DST="$REPO_ROOT/.cursor/skills"
CLAUDE_DST="$REPO_ROOT/.claude/commands"

# 1. Sync skills/ → .cursor/skills/
mkdir -p "$CURSOR_DST"
rsync -a --update --exclude "build.sh" "$SKILLS_SRC/" "$CURSOR_DST/"
echo "✓ skills → .cursor/skills/"

# 2. Sync skills/ → .claude/commands/
mkdir -p "$CLAUDE_DST"
rsync -a --update --exclude "build.sh" "$SKILLS_SRC/" "$CLAUDE_DST/"
echo "✓ skills → .claude/commands/"

echo "build.sh done"
