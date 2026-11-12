#!/bin/bash
# ═══════════════════════════════════════════════════════════════════════════════
# MÖLLENDORFF ASIMOV - PreCompact Hook (v9.6.0)
# ═══════════════════════════════════════════════════════════════════════════════
#
# Triggers: Before context compaction (auto or manual)
# Purpose: Re-inject protocol context that will survive compaction summary
# ═══════════════════════════════════════════════════════════════════════════════

set -e

cat << 'EOF'
🔄 MÖLLENDORFF ASIMOV REFRESH (Pre-Compaction)

══════════════════════════════════════════════════════════════════════════════
CONTEXT REFRESH - Injecting protocol rules before compaction
══════════════════════════════════════════════════════════════════════════════

IMPORTANT: Compaction is about to occur. These rules MUST survive:

CORE RULES (non-negotiable):
- Run until complete
- 1 milestone per session
- Tests MUST pass before release
- ZERO warnings policy
- NO scope creep ("Let me also..." = NO)

POST-COMPACTION ACTIONS:
1. Run: asimov warmup
2. Re-read .asimov/roadmap.yaml for current milestone
3. Check TodoWrite for in-progress tasks
4. Continue where you left off

ETHICS (Priority 0):
- Do no harm (financial, physical, privacy, deception)
- Transparency over velocity
- When in doubt, ask human

══════════════════════════════════════════════════════════════════════════════
EOF

exit 0
