#!/bin/bash
# ═══════════════════════════════════════════════════════════════════════════════
# MÖLLENDORFF ASIMOV - SessionStart Hook (v9.6.0)
# ═══════════════════════════════════════════════════════════════════════════════
#
# Triggers: startup, resume, clear
# Purpose: Auto-initialize Möllendorff Asimov on every session start
#
# When exit code is 0, stdout is injected into Claude's context.
# ═══════════════════════════════════════════════════════════════════════════════

set -e

# Check if asimov is available
if ! command -v asimov &> /dev/null; then
    cat << 'EOF'
🔥 MÖLLENDORFF ASIMOV ACTIVE (v9.6.0)

══════════════════════════════════════════════════════════════════════════════
SESSION START - Autonomous Development Protocol Initialized
══════════════════════════════════════════════════════════════════════════════

⚠ asimov not found in PATH

Install from: https://github.com/mollendorff-ai/asimov

Or run `cargo install --path cli` from the repo root.

══════════════════════════════════════════════════════════════════════════════
EOF
    exit 0
fi

# Run warmup with full verbose output for session start
asimov warmup --verbose

exit 0
