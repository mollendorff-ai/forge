#!/bin/bash
set -e
cat << 'EOF'
🔄 ASIMOV PROTOCOL REFRESH (Pre-Compaction)

══════════════════════════════════════════════════════════════════════════════
CONTEXT REFRESH - Injecting protocol rules before compaction
══════════════════════════════════════════════════════════════════════════════

CORE RULES (non-negotiable):
- 4 hour MAX session duration
- 1 milestone per session
- Tests MUST pass before release
- ZERO warnings policy
- NO scope creep ("Let me also..." = NO)

POST-COMPACTION ACTIONS:
1. Re-read warmup.yaml for full protocol context
2. Re-read sprint.yaml for current milestone
3. Check TodoWrite for in-progress tasks
4. Continue where you left off

CONFUSION PROTOCOL:
If uncertain: STOP → re-read warmup.yaml → re-read sprint.yaml → continue

══════════════════════════════════════════════════════════════════════════════
EOF
exit 0
