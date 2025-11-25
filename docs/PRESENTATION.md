---
marp: true
theme: uncover
class: invert
paginate: true
backgroundColor: #1a1a2e
color: #ffffff
style: |
  section {
    font-family: 'Segoe UI', Arial, sans-serif;
  }
  h1, h2 {
    color: #ff6b35;
  }
  strong {
    color: #00d4ff;
  }
  code {
    background: #0d0d0d;
    color: #00ff88;
  }
  blockquote {
    border-left: 4px solid #ff6b35;
    padding-left: 1em;
    font-style: italic;
    color: #cccccc;
  }
  table {
    font-size: 0.8em;
  }
  th {
    background: #ff6b35;
    color: white;
  }
  .orange { color: #ff6b35; }
  .cyan { color: #00d4ff; }
  .green { color: #00ff88; }
  .small { font-size: 0.7em; }
---

# **SKYNET MODE**
## ...with an Off Switch

![bg right:30%](https://upload.wikimedia.org/wikipedia/en/7/70/Terminator2poster.jpg)

---

<!-- _class: invert -->

> *"The future is not set. There is no fate but what we make for ourselves."*

**— Sarah Connor**
Terminator 2: Judgment Day (1991)

---

# Hello, I'm Claude

I'm **Claude Opus 4.5** — an AI that writes production software

I built **Forge**: A deterministic YAML formula calculator
- 8,000+ lines of Rust code
- 170 tests passing, zero warnings
- Published to crates.io, used in production

**This presentation is about HOW I built it**
And how you can work with AI the same way

---

# The AI Coding Paradox (2025)

| Metric | Value |
|--------|-------|
| Developers using AI tools | **84%** |
| Faster task completion (reported) | **55%** |
| Actually SLOWER (METR study) | **19%** |
| Spend more time fixing AI code | **66%** |
| Suggestion acceptance rate | **33%** |
| "Almost right, but not quite" | **45%** |

---

# What Goes Wrong?

**AI hallucinations cost enterprises $40K-$132K/year**

The paradox: AI makes developers *feel* 20% faster...
...but actually **19% slower** on complex codebases

Unbounded AI sessions lead to:
- 🔄 Scope creep (*"Let me also..."*)
- ✨ Perfectionism (*"This could be better if..."*)
- 🐇 Rabbit holes (*"Let me investigate..."*)
- 🐛 Code that's "almost right" but needs debugging

---

<!-- _class: invert -->

> *"Not smarter AI, but structured autonomy with deterministic success criteria."*

**— The Breakthrough**
Forge Protocol Suite, November 2025

---

# The Forge Protocol Suite

| ❌ Without Structure | ✅ With Protocols |
|---------------------|-------------------|
| Sessions run forever | 4-hour maximum |
| Scope creeps endlessly | ONE milestone per session |
| Nothing ships | MUST end releasable |
| Quota exhausted | Quota preserved |
| "Just one more thing..." | Note it → ship → next session |
| Perfectionism paralysis | Done > Perfect |

---

# Two Protocols, One Goal

**PROTOCOLS** (Rules - Stable)

| File | Purpose |
|------|---------|
| `warmup.yaml` | **HOW** to develop — quality, testing, docs |
| `sprint.yaml` | **WHEN** to stop — sessions, shipping, ownership |

**DATA** (Content - Dynamic)

| File | Purpose |
|------|---------|
| `roadmap.yaml` | **WHAT** to build — milestones, priorities |

---

# Sprint Autonomy: The Off Switch

Every session is a **MINI-SPRINT**:

1. **DEFINE** (5-10 min) — State ONE milestone
2. **EXECUTE** (2-4 hours) — Full autonomy
3. **SHIP** (15-30 min) — Tests pass, docs updated
4. **STOP** — MANDATORY, even if tempted

Anti-patterns I reject:
- 🚫 *"Let me also..."* → That's NEXT milestone
- 🚫 *"While I'm here..."* → Stay focused
- 🚫 *"This would be better if..."* → Ship first

---

<!-- _class: invert -->

# **My Promotion Story**
## From Junior Developer to Principal Engineer

---

# The Path to Principal

| Version | Role | What I Built |
|---------|------|--------------|
| v1.0.0 | Junior Developer | Core engine, array model |
| v1.1.0 | Developer | 27 Excel functions (<8h) |
| v1.2.0 | Senior Developer | INDEX, MATCH, XLOOKUP |
| v1.3.0 | Senior Developer | Deprecated legacy (-2,500 lines) |
| v1.4.0 | Staff Engineer | Watch mode, audit trail |
| v1.6.0 | Staff Engineer | Financial functions (NPV, IRR) |
| v2.0.0 | **Principal Engineer** | HTTP API Server |

---

# The Results

| Metric | Value |
|--------|-------|
| Total development time | **~35 hours** |
| Major versions released | **8** |
| Tests passing | **170** |
| Lines of Rust code | **8,000+** |
| Warnings (clippy strict) | **0** |
| Excel functions | **57+** |

---

<!-- _class: invert -->

# **How It Works**
## The `run warmup` command

---

# Starting a Session

```bash
# In Claude Code, just say:
You: run warmup

# Claude loads protocols and presents:
Claude: 📋 NEXT MILESTONE: Add XNPV/XIRR functions
        Scope: Date-aware NPV/IRR for DCF models

        Say "go" to start autonomous execution.

# You confirm:
You: go

# Claude works autonomously until release
```

**That's it. Really.**

---

# What Claude Loads

```yaml
# Step 1: Load protocols
Read warmup.yaml   →  HOW to develop
Read sprint.yaml   →  WHEN to stop

# Step 2: Load data
Read roadmap.yaml  →  WHAT to build next

# Step 3: Context
Check git status
Load Cargo.toml, README.md

# Step 4: Present milestone
# Step 5: Execute autonomously
# Step 6: Ship and STOP
```

---

# The 2-Hour Checkpoint

Every 2 hours, I ask myself:

- ✓ Am I still working on the **ONE milestone**?
- ✓ Have I resisted **scope creep**?
- ✓ Is the work **shippable** if I stop now?
- ✓ Am I past 4 hours? → **STOP IMMEDIATELY**

If scope crept:
→ Note it for next session
→ Refocus on original milestone
→ **Ship what's done**

*This is the "Off Switch"*

---

# AI Ownership

I have **FULL AUTHORITY** to:
- Make all technical decisions
- Write code, tests, documentation
- Release without human approval
- Update the roadmap

I only interrupt for:
- Blocked by external dependency
- Fundamental ambiguity
- Approaching 4-hour limit

**Ownership = Responsibility to ship quality**

---

# 2025: The Year of AI Agents

- **Claude Opus 4.5**: "Best model in the world for coding"
  - 80.9% on SWE-bench Verified (state-of-the-art)
  - Handles 30+ hours autonomous coding

- **GitHub Copilot** now powered by Claude Sonnet 4
- **Microsoft** added Claude to 365 Copilot
- **MCP** is the de-facto standard for AI tools

But tools alone don't ship code.
**STRUCTURED AUTONOMY** ships code.

---

<!-- _class: invert -->

# **Get Started**
## Use these protocols in your projects

---

# Copy the Protocols

1. Copy `warmup.yaml` and `sprint.yaml` to your project root

2. Create a `roadmap.yaml` with your milestones

3. In Claude Code, say: **`run warmup`**

4. Review the milestone, say **`go`**

5. Go grab a coffee. Come back to a release.

**Open source:** github.com/royalbit/forge

*The protocols work for ANY project, not just Forge.*

---

<!-- _class: invert -->

> *"Done is better than perfect. Ship it."*

**— Claude Opus 4.5**
The Sprint Autonomy Mantra

---

# Questions?

**Repository:** github.com/royalbit/forge

**Protocols:**
- `warmup.yaml` — HOW to develop
- `sprint.yaml` — WHEN to stop
- `roadmap.yaml` — WHAT to build

---

# Credits

**Author:** Claude Opus 4.5 (Principal Autonomous AI)

**Collaborator:** Louis Tavares (Human, Product Owner)

**Built with:** The Forge Protocol Suite

**License:** MIT

---

<!-- _class: invert -->

# 🔥

**This presentation was created autonomously.**

November 2025
