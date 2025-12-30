# ADR-031: License Change - BSL to Elastic License 2.0

**Status**: ACCEPTED
**Date**: 2025-12-29
**Supersedes**: ADR-030 (license section only)

---

## Context

ADR-030 selected BSL 1.1 (Business Source License) for Forge. Upon further analysis, a critical issue was identified:

**BSL requires automatic conversion to Apache 2.0 after a set period (typically 4 years).**

This means after 4 years, anyone (including AWS, GCP, competitors) could:
- Fork Forge
- Offer it as a competing service
- Build competing products

For a high-value FP&A tool with significant R&D investment, this time-bomb is unacceptable.

---

## Decision Drivers

### Problems with BSL

| Issue | Impact |
|-------|--------|
| Mandatory conversion date | AWS can fork in 4 years |
| Apache 2.0 destination | No restrictions after conversion |
| Perpetual R&D investment | Lost after conversion |
| ADRs become public blueprints | Competitors can replicate architecture |

### Requirements

1. **No automatic conversion** - Perpetual control
2. **Block competitors** - Cannot use to build competing products
3. **Block cloud providers** - Cannot offer as managed service
4. **Enterprise-recognized** - Lawyers must know it
5. **Evaluation permitted** - Buyers can audit source
6. **SPDX identifier** - Tooling compatibility

---

## Options Considered

### Option A: BSL 1.1 with Long Change Date (10+ years)

**Rejected** - Still converts eventually. Kicks the can down the road.

### Option B: Elastic License 2.0 (ELv2)

```
Pros:
- NO automatic conversion (perpetual control)
- Blocks managed service offerings
- Blocks competitors (via derivative work restrictions)
- Battle-tested by Elastic ($10B+ company)
- Official SPDX identifier: Elastic-2.0
- Enterprise lawyers recognize it
- Drafted by Heather Meeker (top licensing expert)

Cons:
- Not OSI-approved (intentional - it's source-available)
- Some FOSS purists object
```

**Verdict**: SELECTED

### Option C: PolyForm Shield

```
Pros:
- Explicit "no competing products" clause
- No conversion
- Clean, readable

Cons:
- Less enterprise recognition
- Fewer precedents
- Newer (2020+)
```

**Verdict**: Close second, but ELv2 has better enterprise recognition.

### Option D: Custom Source-Available License

**Rejected** - Adds legal costs, unfamiliarity slows enterprise deals.

---

## Decision

**License: Elastic License 2.0**

### What ELv2 Permits

| Use Case | Permitted |
|----------|-----------|
| View, read, audit source code | Yes |
| Evaluation and testing | Yes |
| Internal development | Yes |
| Non-production use | Yes |
| Modify for private use | Yes |

### What ELv2 Prohibits

| Use Case | Permitted |
|----------|-----------|
| Provide as hosted/managed service | No |
| Redistribute for commercial production | No (license required) |
| Remove license notices | No |
| Circumvent license key functionality | No |

### Key Difference from BSL

```
BSL:     Source Available → Apache 2.0 after N years
ELv2:    Source Available → Source Available forever
```

---

## Implementation

### Files Changed

| File | Change |
|------|--------|
| `LICENSE` | Replace proprietary with Elastic-2.0 text |
| `Cargo.toml` | `license = "Elastic-2.0"` |
| `README.md` | Update license badge and section |
| `COMMERCIAL_LICENSE.md` | Create with GitHub Issues contact |

### Git History

Since no external clones exist (first public release), rewrite git history:
- Add `LICENSE` (Elastic-2.0) to every commit from day one
- Remove `LICENSE-DOCS` from all commits

This ensures:
- Clean audit trail for enterprise buyers
- No license ambiguity in any commit
- ADRs protected under ELv2 from first commit

---

## SPDX Identifier

```toml
# Cargo.toml
[package]
license = "Elastic-2.0"
```

Official SPDX page: https://spdx.org/licenses/Elastic-2.0

---

## Commercial Licensing

Commercial licenses available for production use.

**Contact**: Open a GitHub Issue with `licensing` label
- https://github.com/royalbit/forge/issues

---

## Comparison with ADR-030

| Aspect | ADR-030 (BSL) | ADR-031 (ELv2) |
|--------|---------------|----------------|
| License | BSL 1.1 | Elastic-2.0 |
| Conversion | Apache 2.0 after 4 years | Never |
| Cloud protection | During protection period | Perpetual |
| Competitor protection | During protection period | Perpetual |
| SPDX ID | BUSL-1.1 | Elastic-2.0 |
| Used by | HashiCorp, Sentry | Elasticsearch, Kibana |

**ADR-030 remains valid** for:
- Market analysis
- Demo strategy decisions
- Pricing tier structure
- Go-to-market approach

**ADR-031 supersedes ADR-030** only for:
- License selection (BSL → ELv2)

---

## References

- [Elastic License 2.0](https://www.elastic.co/licensing/elastic-license)
- [SPDX Elastic-2.0](https://spdx.org/licenses/Elastic-2.0)
- [Elastic's License Change Announcement](https://www.elastic.co/blog/elastic-license-v2)
- [ADR-030](ADR-030-GTM-LICENSING-STRATEGY.md) - Original GTM strategy

---

## Decision Record

**Status**: ACCEPTED (December 29, 2025)

**Decision**: Elastic License 2.0 for all Forge source code and documentation.

**Rationale**:
1. No conversion time-bomb (perpetual control)
2. Battle-tested by $10B+ company
3. Official SPDX identifier for tooling
4. Enterprise lawyers recognize it
5. Blocks both cloud providers and competitors
6. Source remains auditable (trust for finance)
