#!/bin/bash
# ═══════════════════════════════════════════════════════════════════════════════
# ROYALBIT ASIMOV - SessionStart Hook (v8.1.0)
# ═══════════════════════════════════════════════════════════════════════════════
#
# Triggers: startup, resume, clear
# Purpose: Auto-initialize RoyalBit Asimov on every session start
#
# When exit code is 0, stdout is injected into Claude's context.
#
# v8.1.0: project.yaml added for project context (ADR-032)
# v8.0.0: Protocols are hardcoded in binary - run `asimov warmup` to load
#
# Protocol: https://github.com/royalbit/asimov
# ═══════════════════════════════════════════════════════════════════════════════

set -e

cat << 'EOF'
🔥 ROYALBIT ASIMOV ACTIVE (v8.1.0)

══════════════════════════════════════════════════════════════════════════════
SESSION START - Autonomous Development Protocol Initialized
══════════════════════════════════════════════════════════════════════════════

IMMEDIATE ACTIONS REQUIRED:
1. Run: asimov warmup (loads hardcoded protocols + validates)
2. Read .asimov/roadmap.yaml for current version and next milestone
3. Present next milestone to user
4. Wait for "go" to start autonomous execution

CORE RULES (non-negotiable):
- 4 hour MAX session duration
- 1 milestone per session
- Tests MUST pass before release
- ZERO warnings policy

Say "go" to start autonomous execution.
Say "skip" to pick a different milestone.
Say "plan" to discuss approach first.

ASIMOV MODE:
When user says "asimov mode", display:
══════════════════════════════════════════════════════════════════════════════
🤖 ASIMOV MODE - AUTONOMOUS EXECUTION ENGAGED
══════════════════════════════════════════════════════════════════════════════
Then proceed with full autonomous execution for 4h MAX or until roadmap exhausted.

══════════════════════════════════════════════════════════════════════════════
EOF

exit 0
