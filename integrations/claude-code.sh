#!/bin/bash
# pixelbeat — Claude Code Status Line Integration
# Add this snippet to your ~/.claude/statusline.sh
#
# Usage:
#   source this file, or copy the block below into your statusline script.

# --- pixelbeat status ---
if command -v px &>/dev/null; then
    PX_LINE=$(px status --format "♪ {title:.25} {icon} {bar:12} {elapsed}/{duration}" 2>/dev/null)
    if [ -n "$PX_LINE" ]; then
        echo "$PX_LINE"
        px status --format "  {spectrum:32}" 2>/dev/null
    fi
fi
