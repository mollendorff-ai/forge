# SR&ED Documentation Guide

**Scientific Research & Experimental Development (Canada)**

Tax credits for R&D work (refundable in some cases).

## Eligibility

Technical challenges, systematic investigation, technological advancement.

## When to Document

- **BEFORE** committing new algorithms or data structures
- **BEFORE** committing experimental approaches or optimizations
- **BEFORE** committing solutions to technical uncertainties
- After resolving performance issues
- After experimenting with alternative approaches
- When creating novel abstractions or patterns

## What to Document

### Technical Challenge
- What problem are we solving?
- Why is it technically uncertain?
- What makes it non-trivial/non-obvious?

### Hypothesis
- What approach did we try?
- What alternatives did we consider?
- Why did we choose this approach?

### Experiment
- What did we implement?
- What tests did we run?
- What measurements did we take?

### Results
- Did it work? Why or why not?
- What did we learn?
- Were there unexpected findings?

### Advancement
- What new capability did we create?
- How does it advance the state of the art?
- What can users now do that they couldn't before?

## Qualifying Activities

### Yes (Qualifies)
- Algorithm design and optimization
- Performance analysis and improvements
- Experimental testing approaches (property-based, mutation, fuzzing)
- Resolving technical uncertainties
- Creating novel data structures and abstractions
- Dependency resolution algorithms
- Type system design

### No (Does Not Qualify)
- Routine coding (following established patterns)
- UI design and styling
- Documentation writing (unless documenting research)
- Bug fixes for simple typos
- Standard library integration

## Workflow

1. Identify if work qualifies for SR&ED (see above)
2. If YES: Document in `docs/internal/SRED_RESEARCH_LOG.md` DURING development
3. Include: Challenge, Hypothesis, Experiment, Results, Advancement
4. Be specific about technical challenges and alternatives considered
5. Link to commits, test results, benchmarks
6. THEN commit code

## Example Entry Titles

- "Entry X: Aggregation Formula Evaluation (Phase 2 Part 2)"
- "Entry X: Zero-Copy String Optimization"
- "Entry X: Property-Based Testing for Formula Invariants"

---

**This documentation = real money in tax credits. Don't skip it!**
