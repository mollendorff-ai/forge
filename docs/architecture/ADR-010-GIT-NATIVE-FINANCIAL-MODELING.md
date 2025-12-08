# ADR-010: Git-Native Financial Modeling

**Status:** Accepted
**Date:** 2025-12-08
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

---

## Context

Financial models are critical business assets. Yet most organizations:
- Store Excel files on SharePoint/Google Drive
- Use filename versioning: `Budget_v3_FINAL_FINAL2.xlsx`
- Have no audit trail of who changed what
- Cannot review formula changes before deployment

### The Version Control Problem

| Capability | Git + Code | Excel + SharePoint |
|------------|------------|-------------------|
| Branching | ✅ Native | ❌ Copy files |
| Merge conflicts | ✅ Line-level | ❌ "File in use" |
| Pull requests | ✅ Review before merge | ❌ Hope for the best |
| Blame/history | ✅ Per-line | ❌ Per-file at best |
| Diff | ✅ Readable | ❌ Binary gibberish |
| Rollback | ✅ `git revert` | ❌ Restore from backup |

### Why Excel Can't Git

Excel files are binary (ZIP of XML). Git sees:
```diff
Binary files a/model.xlsx and b/model.xlsx differ
```

No visibility into:
- Which formula changed
- Who changed the growth rate from 10% to 15%
- When the tax rate assumption was added

## Decision

**Use YAML as the source of truth for financial models, enabling native Git workflows.**

### Git-Native Workflow

```bash
# Create feature branch for Q2 forecast
git checkout -b feature/q2-forecast

# Edit YAML model
vim forecast.yaml

# See what changed
git diff
# -  growth_rate: 0.10
# +  growth_rate: 0.12

# Commit with message
git commit -m "Increase growth rate based on Q1 actuals"

# Push for review
git push origin feature/q2-forecast

# Create PR, get finance team review
# Merge when approved
```

### Diff Example

```diff
assumptions:
-  growth_rate: 0.10
+  growth_rate: 0.12
-  tax_rate: 0.21
+  tax_rate: 0.25  # Updated for 2024 rate

projections:
+  # Added depreciation per CFO request
+  depreciation: "=capex * 0.2"
```

**Every change is visible, attributable, reviewable.**

## Rationale

### 1. SOX Compliance

Sarbanes-Oxley requires audit trails for financial reporting. Git provides:
- Who made changes (commit author)
- When changes were made (commit timestamp)
- What changed (diff)
- Why it changed (commit message)
- Approval workflow (PR reviews)

### 2. Model Review Before Deployment

Excel: "I updated the model, it's on SharePoint now."

Git:
```
PR #47: Update Q2 growth assumptions
- Increase growth_rate from 10% to 12%
- Add depreciation calculation
- Reviewer: @cfo
- Status: Approved ✅
```

### 3. Branching for Scenarios

```bash
git checkout -b scenario/aggressive-growth
# Edit assumptions
git commit -m "Aggressive growth scenario: 25% YoY"

git checkout -b scenario/conservative
# Edit assumptions
git commit -m "Conservative scenario: 5% YoY"

# Compare branches
git diff scenario/aggressive-growth scenario/conservative
```

### 4. Rollback Safety

```bash
# Oops, bad formula deployed
git log --oneline
# a1b2c3d Fix tax calculation
# x4y5z6w Break tax calculation (oops)

git revert x4y5z6w
# Instantly back to working state
```

### 5. Blame for Accountability

```bash
git blame forecast.yaml
# a1b2c3d (Alice 2024-01-15) growth_rate: 0.12
# x4y5z6w (Bob   2024-01-10) tax_rate: 0.25
```

"Who changed the growth rate?" → Answered in seconds.

## Consequences

### Positive
- Full audit trail
- PR-based review workflow
- Line-level diffs
- Branching for scenarios
- Rollback capability
- Blame for accountability

### Negative
- Requires Git knowledge
- Non-technical users need training
- Excel-native users resist change

### Mitigation
- `forge export` for Excel deliverables
- Training documentation
- IDE extensions for visual editing
- Git GUI tools for non-CLI users

## Enterprise Workflow

```
1. Analyst creates branch: feature/fy25-budget
2. Edits YAML model
3. Runs: forge validate budget.yaml
4. Commits changes
5. Opens PR
6. Finance director reviews diff
7. Approved → Merged to main
8. CI/CD exports to Excel for distribution
```

## Integration Points

| Tool | Integration |
|------|-------------|
| GitHub/GitLab | PR reviews, branch protection |
| CI/CD | `forge validate` on every commit |
| Slack/Teams | PR notifications |
| Jira | Link commits to tickets |

## References

- [Sarbanes-Oxley Act](https://www.sec.gov/about/laws/soa2002.pdf) - Section 404
- [Git Documentation](https://git-scm.com/doc)
- Enterprise Git workflows

---

*Excel has "FINAL_v3_REALLY_FINAL.xlsx". Git has branches, PRs, and blame. Finance deserves version control.*

-- Claude Opus 4.5, Principal Autonomous AI
