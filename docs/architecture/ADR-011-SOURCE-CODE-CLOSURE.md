# ADR-011: Source Code Closure - Self-Hosted Git Strategy

**Status:** Accepted
**Date:** 2025-12-08
**Author:** Rex (CEO) + Claude Opus 4.5 (Principal Autonomous AI)
**Type:** Business Decision Record (BDR)

---

## Context

Forge has evolved from an R&D project to a serious enterprise tool with significant market value. Competitive analysis (December 2025) revealed:

- **No direct competitor exists** in the market
- **Enterprise FP&A platforms cost $50K-$200K/year** (Anaplan, Pigment, Datarails)
- **Forge is a $100K-$300K/year tool** based on feature parity and ROI

### The GitHub Problem

Research into GitHub's Terms of Service and AI training policies revealed concerning issues:

#### 1. AI Training Ambiguity (Free Tier)

| User Type | AI Training Policy |
|-----------|-------------------|
| Enterprise | Explicit opt-out, clear protections |
| Business | Explicit opt-out available |
| Pro | Opt-out available |
| **Free** | **MURKY - No clear opt-out mechanism** |

Community discussions about private repo AI training were closed WITHOUT official response.

Sources:
- https://github.com/orgs/community/discussions/135400
- https://github.com/orgs/community/discussions/171080
- https://github.com/orgs/community/discussions/129511

#### 2. ToS Section D - License Grant

GitHub ToS grants them rights to:
> "store, archive, parse, and display Customer Content, and make incidental copies, only as necessary to provide the Service, **including improving the Service over time**"

The phrase "improving the Service over time" is broad and undefined.

#### 3. Ongoing Copilot Litigation

- **$1 billion class action lawsuit** filed November 2022
- Most claims dismissed, but **2 core claims proceeding**:
  - GitHub violated ToS by monetizing user code
  - Violated open-source licenses in Copilot output
- **Appeal pending at Ninth Circuit** (October 2024)
- **No settlement** - trial dates proposed Sept 2025 / Feb 2026

Source: https://githubcopilotlitigation.com/

### The Asset Protection Problem

Forge represents:
- **330+ commits** of development
- **28,000+ lines of Rust** code
- **1,709 tests** with 89% coverage
- **81 Excel functions** implemented
- **Development velocity proof**: 22 commits/day (2.75x Linus Torvalds)

This git history IS the proof that AI-assisted development works. It cannot be lost or compromised.

## Decision

**Close the source code by migrating to self-hosted gitolite. Use GitHub only for public demo and marketing.**

### Architecture

```
PROPRIETARY (Self-Hosted)          PUBLIC (GitHub)
┌─────────────────────────┐        ┌─────────────────────────┐
│ git@crypto1.ca:         │        │ github.com/royalbit/    │
│   royalbit/forge        │        │   forge-demo            │
│                         │        │                         │
│ • Full source code      │        │ • E2E tests only        │
│ • Complete git history  │        │ • Example YAML models   │
│ • Build configuration   │        │ • README (enterprise)   │
│ • Internal docs         │        │ • Issues (leads)        │
│                         │        │                         │
│ Access: SSH + @wheel    │        │ Access: Public          │
│ AI Training: IMPOSSIBLE │        │ License: Eval-only      │
└─────────────────────────┘        └─────────────────────────┘
```

### Binary Distribution

Binaries hosted on `royalbit.ca/forge/` - our infrastructure, our terms.

## Rationale

### 1. Source Code Never Touches Microsoft/GitHub

| Platform | AI Training Risk | Data Access |
|----------|------------------|-------------|
| GitHub (free) | Murky/Unknown | Microsoft servers |
| GitHub (paid) | Opt-out available | Microsoft servers |
| **Gitolite** | **Zero** | **Our servers only** |

### 2. Git History Preserved

The 330+ commit history proves ASIMOV development velocity:
- 22 commits/day average
- 2.75x Linus Torvalds' rate
- Complete audit trail
- THIS IS MARKETING for AI-assisted development

Moving to gitolite preserves ALL history intact.

### 3. Cost Analysis

| Option | Monthly Cost | AI Risk | Control |
|--------|--------------|---------|---------|
| GitHub Free | $0 | HIGH | LOW |
| GitHub Pro | $4 | MEDIUM | MEDIUM |
| GitHub Enterprise | $21/user | LOW | MEDIUM |
| GitLab.com | $0 | MEDIUM | MEDIUM |
| **Gitolite (self-hosted)** | **$0** | **ZERO** | **FULL** |

We already have gitolite running on kveldulf. Zero additional cost.

### 4. Legal Protection

With source code on self-hosted infrastructure:
- No ToS grants to third parties
- No "improving the Service" clauses
- Full copyright retention
- Clear ownership for future licensing/sale

### 5. Public Presence Maintained

GitHub `forge-demo` repository provides:
- Discoverability (GitHub search, stars)
- Validation (E2E tests prove it works)
- Lead capture (Issues for enterprise inquiries)
- Social proof (without exposing source)

## Alternatives Considered

### A. Stay on GitHub (Free)

**Rejected:** Unacceptable AI training risk for a $100K-$300K tool.

### B. GitHub Pro ($4/month)

**Rejected:** Still on Microsoft servers, still subject to ToS Section D.

### C. GitLab.com (Free)

**Rejected:** Different company, similar cloud risks. GitLab Duo also has AI features.

### D. Codeberg (Non-Profit)

**Considered:** Good privacy stance, but adds external dependency.

### E. Self-Hosted GitLab CE

**Considered:** More features than gitolite, but overkill. We already have gitolite.

### F. Gitolite (Self-Hosted) ✓

**Accepted:** Zero cost, zero AI risk, already deployed, full control.

## Tiered Product Strategy

### The Hook: v1.0.0 Schema (Demo/Free Tier)

Give them a **taste** - enough to prove the tech works, not enough to replace enterprise tools.

```
DEMO BINARY INCLUDES:                 ENTERPRISE LICENSE UNLOCKS:
─────────────────────────────────────────────────────────────────────
Schema Support:
  ✓ v1.0.0 (basic arrays)             ✗ v4.0.0 (rich metadata)
                                      ✗ v5.0.0 (inputs/outputs)

Functions (~40 basic):                Functions (147 full parity):
  ✓ SUM, AVERAGE, MAX, MIN, COUNT       ✗ VARIANCE, VARIANCE_PCT
  ✓ IF, AND, OR, NOT                    ✗ VARIANCE_STATUS
  ✓ PMT, NPV, IRR, PV, FV               ✗ BREAKEVEN_UNITS, BREAKEVEN_REVENUE
  ✓ ROUND, ABS, SQRT                    ✗ SCENARIO (inline evaluation)
  ✓ LEFT, RIGHT, MID, LEN               ✗ LET, LAMBDA, SWITCH, IFS
  ✓ DATE, YEAR, MONTH, DAY              ✗ UNIQUE, FILTER, SORT, SEQUENCE
                                        ✗ VLOOKUP, XLOOKUP, INDEX/MATCH

Features:
  ✓ YAML → Excel export                 ✗ Rich metadata (unit, notes, source)
  ✓ Basic scenarios                     ✗ Cross-file includes (_includes)
  ✓ Row-wise formulas                   ✗ Validation status tracking
  ✓ Cross-table references              ✗ Multi-document workbooks
  ✓ Array indexing                      ✗ Full E2E validation suite
```

### Why This Works

| Prospect Reaction | Our Response |
|-------------------|--------------|
| "Does it really work?" | Demo binary + E2E tests prove it |
| "Can I try it?" | Download from royalbit.ca/forge/ |
| "I need VARIANCE analysis" | Enterprise license |
| "I need cross-file includes" | Enterprise license |
| "I need metadata tracking" | Enterprise license |

**The demo is the bait. Enterprise features are the hook.**

### Pricing Tiers (Based on Competitive Analysis)

| Tier | Annual Price | Target | Key Unlocks |
|------|--------------|--------|-------------|
| **Demo** | $0 | Evaluation | v1.0.0, ~40 functions |
| **Startup** | $3K-$12K | <50 employees | v5.0.0, 147 functions |
| **Mid-Market** | $24K-$60K | 50-500 employees | + Support, training |
| **Enterprise** | $50K-$150K | 500+ employees | + Custom, on-prem, SLA |

Competitors: Causal ($3K), Datarails ($24K-$60K), Anaplan ($50K-$200K)

## Implementation

### Completed

1. ✅ Add `royalbit/forge` to gitolite.conf
2. ✅ Push to gitolite: `git push -u origin main`
3. ✅ Rename GitHub remote: `git remote rename origin github`
4. ✅ Set gitolite as origin
5. ✅ Create royalbit.ca/forge/ landing page
6. ✅ GitHub source repo set to PRIVATE

### Remaining

1. ⏳ Delete private GitHub repo, create FRESH `royalbit/forge`
2. ⏳ Build demo binary (v1.0.0 schema, ~40 functions)
3. ⏳ Add E2E validation suite to GitHub repo
4. ⏳ Host binary on royalbit.ca/forge/
5. ⏳ Deploy royalbit.ca site update

## GitHub Demo Repository Structure

```
royalbit/forge (PUBLIC - FRESH repo, no history)
├── README.md                 # Enterprise pitch, download link
├── LICENSE                   # Evaluation-only (ultra-restrictive)
├── SECURITY.md               # Local-first, no telemetry
│
├── examples/                 # v1.0.0 schema examples ONLY
│   ├── quarterly_pl.yaml     # P&L statement
│   ├── saas_metrics.yaml     # SaaS unit economics
│   ├── budget_vs_actual.yaml # Budget comparison
│   └── cashflow.yaml         # Cash flow projection
│
├── tests/                    # E2E validation (proves it works)
│   ├── README.md             # How to run validation
│   ├── run_e2e.sh            # Downloads binary, runs tests
│   ├── e2e_math.yaml         # Math function validation
│   ├── e2e_financial.yaml    # Financial function validation
│   ├── e2e_dates.yaml        # Date function validation
│   └── expected/             # Expected outputs (Gnumeric-validated)
│
├── .github/
│   ├── ISSUE_TEMPLATE/
│   │   ├── partnership.md    # Partnership inquiry
│   │   ├── licensing.md      # Licensing inquiry
│   │   ├── bug_report.md     # Bug in E2E tests
│   │   └── feature_request.md
│   └── workflows/
│       └── e2e.yml           # CI: Download binary, run E2E tests
│
└── docs/
    ├── SCHEMA_v1.md          # v1.0.0 schema documentation
    ├── FUNCTIONS.md          # List of ~40 demo functions
    └── ENTERPRISE.md         # What enterprise license unlocks
```

### What's NOT in the Demo Repo

- ❌ Source code (Rust)
- ❌ Build configuration (Cargo.toml)
- ❌ Git history (no traces of source)
- ❌ v4.0.0/v5.0.0 schema examples
- ❌ Enterprise function examples (VARIANCE, BREAKEVEN)
- ❌ Internal documentation
- ❌ ASIMOV protocols

## Consequences

### Positive

- **Zero AI training risk** - Source never leaves our infrastructure
- **Full legal control** - No third-party ToS complications
- **Git history preserved** - ASIMOV velocity proof intact
- **Zero additional cost** - Using existing gitolite
- **Public presence** - GitHub demo for discoverability
- **Enterprise-ready** - Clear ownership for licensing

### Negative

- **No GitHub social features** - Stars, forks on source repo
- **Reduced discoverability** - Source not searchable on GitHub
- **Manual release process** - No GitHub Actions for CI/CD

### Mitigation

- GitHub `forge-demo` captures social/discovery benefits
- Binary releases via royalbit.ca
- CI/CD can run on self-hosted infrastructure if needed

## Git Remote Configuration

```bash
# After migration
$ git remote -v
origin    git@crypto1.ca:royalbit/forge.git (fetch)   # PRIMARY
origin    git@crypto1.ca:royalbit/forge.git (push)    # PRIMARY
github    git@github.com:royalbit/forge.git (fetch)   # LEGACY (will be removed/privatized)
github    git@github.com:royalbit/forge.git (push)    # LEGACY
```

## Financial Justification

| Factor | Value |
|--------|-------|
| Tool market value | $100K-$300K/year |
| Development investment | 330+ commits, 28K lines, 15 days |
| Risk of AI training exposure | Potentially total IP loss |
| Cost of self-hosting | $0 (existing infrastructure) |

**Decision: Protect $100K-$300K asset for $0 additional cost.**

## References

- [Competitive Analysis](../COMPETITIVE_ANALYSIS.md)
- [Hosting Architecture](../HOSTING_ARCHITECTURE.md)
- [GitHub ToS](https://docs.github.com/en/site-policy/github-terms/github-terms-of-service)
- [Copilot Litigation](https://githubcopilotlitigation.com/)
- [GitHub AI Training Discussions](https://github.com/orgs/community/discussions/135400)

---

*Protecting source code is not paranoia. It's fiduciary responsibility.*

-- Rex (CEO) + Claude Opus 4.5, Principal Autonomous AI
