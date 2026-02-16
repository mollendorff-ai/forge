# ADR-030: Go-To-Market and Licensing Strategy

**Status**: ACCEPTED
**Date**: 2025-12-29
**Decision Makers**: Möllendorff Group Inc.

---

> **Note:** The license section of this ADR was superseded by ADR-031-LICENSE-ELASTIC-2.0.md

## Context

Forge needs a go-to-market strategy that balances:
1. Developer adoption (visibility, trust)
2. Revenue protection (prevent AWS/GCP fork)
3. Market positioning (AI infrastructure for finance)
4. Demo strategy (feature-limited vs time-limited)

The AI orchestration market is saturating (5,879 GitHub repos), while FP&A has no open-source alternatives. Forge's unique position as "AI-native FP&A infrastructure" requires a strategy that leverages both markets.

---

## Decision Drivers

### Market Analysis (See [MARKET_ANALYSIS.md](../MARKET_ANALYSIS.md))

| Factor | AI Orchestration | Forge (FP&A) |
|--------|-----------------|--------------|
| Competitors | 5,879 repos | **Zero** |
| Open source threat | EXTREME | None |
| Exit comps | $1.25B (LangChain) | **$10.7B** (Anaplan) |
| Pricing power | Low | **High** |

### Current State

- **forge**: 173 functions, MCP server, Monte Carlo, full FP&A
- **forge-demo**: 48 functions, no MCP, no Monte Carlo, no FP&A functions
- **forge-e2e**: Gnumeric/R validation suite (separate repo)

---

## Options Considered

### Option A: Permissive Open Source (MIT/Apache)

```
Pros:
- Maximum adoption
- Community contributions
- Trust through transparency

Cons:
- AWS/GCP can fork and compete
- Zero pricing power
- Race to bottom (see: LangChain commoditization)
```

**Verdict**: REJECTED - Gives away the moat without protection.

### Option B: Open Core (MIT core + Proprietary enterprise)

```
Pros:
- Community builds on core
- Enterprise features protected

Cons:
- AWS can still fork the core
- Split community
- Feature decisions become political
```

**Verdict**: REJECTED - Core still forkable by cloud providers.

### Option C: Source Available (BSL) + Cloud MCP

```
Pros:
- Source visible (trust, audit)
- Commercial use requires license
- Cloud providers cannot compete
- Converts to Apache 2.0 after 4 years
- Used by: HashiCorp, Sentry, CockroachDB

Cons:
- Not OSI-approved "open source"
- Some purists object
```

**Verdict**: RECOMMENDED - Balances visibility with protection.

### Option D: SSPL (MongoDB/Elastic model)

```
Pros:
- Strongest cloud provider protection

Cons:
- Controversial
- Scares enterprise legal teams
- Not OSI-approved
```

**Verdict**: REJECTED - Too aggressive, may slow enterprise adoption.

---

## Demo Strategy Options

### Current: Feature-Limited Teaser (forge-demo)

| Aspect | Current State |
|--------|---------------|
| Functions | 48 of 173 (28%) |
| MCP Server | **Not included** |
| Monte Carlo | **Not included** |
| FP&A Functions | **Not included** |
| Time Limit | None |
| Conversion | Low (best features hidden) |

**Problem**: The differentiating features are hidden:
- MCP integration (AI infrastructure moat)
- Monte Carlo (enterprise FP&A)
- FP&A-native functions (domain expertise)
- Financial functions (NPV, IRR, XIRR)

This is like demoing a Tesla without showing autopilot.

### Alternative A: Time-Limited Full Demo (30 days)

| Aspect | Proposed |
|--------|----------|
| Functions | All 173 |
| MCP Server | **Included** |
| Monte Carlo | **Included** |
| FP&A Functions | **Included** |
| Time Limit | 30 days |
| Conversion | High (full value visible) |

**Pros**:
- Shows full value proposition
- Creates natural conversion point
- Developers can evaluate MCP integration
- Finance teams can test Monte Carlo

**Cons**:
- Time pressure may annoy some developers
- Can be gamed (reinstall)

### Alternative B: Usage-Limited Full Demo

| Aspect | Proposed |
|--------|----------|
| Functions | All 173 |
| Limit | 1,000 calculations |
| Conversion | Medium |

**Cons**: Hard to implement cleanly, gameable.

### Alternative C: Watermarked Full Demo

| Aspect | Proposed |
|--------|----------|
| Functions | All 173 |
| Limit | None |
| Output | "DEMO" watermark in exports |

**Cons**: Watermark acceptable for some use cases, may not convert.

---

## Hedge Fund Investor Perspective

### On Licensing

**BSL is correct** because:

1. **Trust matters in finance** - Source must be auditable
2. **Cloud protection essential** - AWS can't fork and compete
3. **Enterprise-friendly** - BSL is accepted by legal teams
4. **Time-delayed OSS** - Eventually Apache 2.0, builds goodwill

### On Demo Strategy

**Time-limited full demo is correct** because:

1. **Show the moat** - MCP, Monte Carlo, FP&A functions ARE the value
2. **Feature-limited fails** - 48 basic functions = any spreadsheet library
3. **Conversion requires experience** - Finance teams need to test Monte Carlo
4. **AI developers need MCP** - Can't evaluate without it
5. **30 days is industry standard** - Anaplan, Workday do the same

**The question is not "will they pay?" but "have they seen enough value?"**

A feature-limited demo answers: "Is this a spreadsheet library?" (Yes, many exist)
A time-limited full demo answers: "Is this AI-native FP&A?" (Only Forge)

---

## Proposed Decision

### Licensing: BSL (Business Source License)

```
┌─────────────────────────────────────────────────────────────┐
│  BSL Terms                                                  │
│                                                             │
│  • Source code: PUBLIC                                     │
│  • Non-production use: ALLOWED                             │
│  • Production use: REQUIRES LICENSE                        │
│  • Change date: 4 years → Apache 2.0                       │
│  • Additional use grant: Educational/research permitted    │
└─────────────────────────────────────────────────────────────┘
```

### Demo Strategy: Time-Limited Full Demo

```
┌─────────────────────────────────────────────────────────────┐
│  forge-demo → forge-trial                                  │
│                                                             │
│  • All 173 functions                                       │
│  • MCP server included                                     │
│  • Monte Carlo included                                    │
│  • 30-day evaluation period                                │
│  • Requires email for download (lead capture)              │
│  • Clear upgrade path to Pro/Enterprise                    │
└─────────────────────────────────────────────────────────────┘
```

### Pricing Tiers

| Tier | Price | Target |
|------|-------|--------|
| **Trial** | Free (30 days) | Evaluation |
| **Pro** | $500-2K/month | Startups, small teams |
| **Enterprise** | $5K-20K/month | Large enterprises |
| **Cloud MCP** | Usage-based | AI platforms |

---

## Implementation Plan

### Phase 1: Licensing (Week 1-2)

1. Add BSL license to forge repo
2. Update README with license terms
3. Add license headers to source files
4. Create COMMERCIAL_LICENSE.md

### Phase 2: Demo Transition (Week 3-4)

1. Rename forge-demo → forge-trial
2. Add 30-day time limit with license key bypass
3. Include all 173 functions
4. Include MCP server
5. Add lead capture (email for download)
6. Create upgrade flow documentation

### Phase 3: Commercial Infrastructure (Week 5-8)

1. License key generation system
2. Stripe/payment integration
3. Customer portal
4. Usage tracking (Cloud MCP)

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| BSL backlash | Clear messaging: "source available, not OSS" |
| Trial gaming (reinstall) | License key tied to machine fingerprint |
| Slow conversion | Extend trial for qualified leads |
| Enterprise legal concerns | BSL is HashiCorp-proven, provide FAQ |

---

## Success Metrics

| Metric | Target (6 months) |
|--------|-------------------|
| GitHub stars | 1,000 |
| Trial downloads | 500 |
| Trial → Pro conversion | 5% |
| Pro → Enterprise upsell | 20% |
| ARR | $100K |

---

## Open Questions

1. Should forge-demo remain as a permanent free tier (48 functions)?
2. What machine fingerprinting approach for trial?
3. Should educational/research use be perpetually free?
4. Cloud MCP pricing model (per-calculation vs subscription)?

---

## References

- [MARKET_ANALYSIS.md](../MARKET_ANALYSIS.md) - Full competitive analysis
- [BSL License](https://mariadb.com/bsl11/) - Business Source License 1.1
- [HashiCorp BSL Announcement](https://www.hashicorp.com/blog/hashicorp-adopts-business-source-license)
- [Sentry BSL Case Study](https://blog.sentry.io/relicensing-sentry/)

---

## Decision

**Status**: ACCEPTED (December 29, 2025)

**Decision**:
- License: **BSL 1.1** (Business Source License)
- Product: **One version** - full forge with all 173 functions
- Demo: **DEPRECATED** - forge-demo archived, redirects to forge
- Revenue gate: **License required for commercial production use**

**Rationale**:
- One product = 1x maintenance (not 2x)
- Full features visible = shows the moat (MCP, Monte Carlo, FP&A)
- BSL protects commercial value without hiding source
- Source available builds trust (critical for finance)
- Converts to Apache 2.0 after 4 years (goodwill)

**What BSL means**:
- Source code: **OPEN** (visible, auditable, forkable for evaluation)
- Non-production use: **FREE** (evaluation, education, research)
- Commercial production: **REQUIRES LICENSE**
- Not FOSS: BSL is "Source Available", not OSI-approved "Open Source"
- After 4 years: Converts to Apache 2.0 (becomes true FOSS)
