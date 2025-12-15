# ADR-017: Monte Carlo Sequential Execution

## Status

**Accepted**

## Context

Monte Carlo simulation in Forge can be implemented using either:
1. **Sequential execution** - Single-threaded iteration loop
2. **Concurrent execution** - Multi-threaded parallel iterations

This decision affects reproducibility, performance, complexity, and auditability.

## Decision

**Forge Monte Carlo uses sequential execution.**

The simulation loop processes iterations one at a time:

```rust
for i in 0..n {
    let inputs = collect_inputs_for_iteration(i);
    let outputs = evaluator(&inputs);
    store_outputs(outputs);
}
```

## Rationale

### 1. Reproducibility is Non-Negotiable for FP&A

Financial Planning & Analysis requires **exact reproducibility**:
- Audit trails (SOX compliance)
- Model validation reviews
- Board/investor presentations
- Regulatory submissions

Sequential execution with a seed guarantees **bit-for-bit identical results** across runs, machines, and time.

Parallel execution introduces non-deterministic thread scheduling that can affect:
- Floating-point accumulation order
- RNG state across threads
- Result ordering

### 2. Performance is Already Sufficient

| Iterations | Sequential Time | Use Case |
|------------|-----------------|----------|
| 10,000 | ~100ms | Standard FP&A analysis |
| 50,000 | ~500ms | High precision |
| 100,000 | ~1s | Publication-grade |

Sub-second execution is effectively instant for FP&A workflows.

### 3. Latin Hypercube Sampling Already Optimizes Convergence

Forge uses Latin Hypercube Sampling (LHS) by default, which converges **5-10x faster** than pure Monte Carlo. This reduces the iteration count needed for equivalent precision, making parallelization unnecessary.

### 4. Industry Standard Practice

| Tool | Default Execution | Rationale |
|------|-------------------|-----------|
| @RISK (Palisade/Lumivero) | Sequential | Reproducibility |
| Crystal Ball (Oracle) | Sequential | Reproducibility |
| ModelRisk (Vose Software) | Sequential | Reproducibility |

The FP&A industry uniformly prioritizes reproducibility over raw speed.

### 5. Simplicity Reduces Risk

Sequential code is:
- Easier to debug
- Easier to audit
- Free of race conditions
- Free of synchronization bugs

**In financial software, boring is good.**

## Consequences

### Positive
- Guaranteed reproducibility with seed
- Simple, auditable codebase
- No threading bugs possible
- Matches industry standard tools

### Negative
- Cannot utilize multiple CPU cores
- Large iteration counts (1M+) will be slower than parallel

### Mitigations
- LHS provides 5-10x convergence speedup (fewer iterations needed)
- 100K iterations in ~1s is sufficient for all FP&A use cases
- If extreme performance needed, optimize formula evaluation first

## Alternatives Considered

### Parallel with Rayon

```rust
samples.par_chunks(chunk_size)
    .map(|chunk| evaluate_chunk(chunk))
    .collect()
```

**Rejected because:**
- Reproducibility requires deterministic chunk assignment and per-chunk RNG seeding
- Adds complexity for marginal benefit
- Not needed given current performance

### Optional Parallel Flag

```yaml
monte_carlo:
  parallel: true  # opt-in
```

**Rejected because:**
- Two code paths to maintain and test
- Users may enable it without understanding reproducibility implications
- YAGNI - no demonstrated need

## References

1. **Latin Hypercube Sampling (Original Paper)**
   McKay, M.D., Beckman, R.J., Conover, W.J. (1979). "Comparison of Three Methods for Selecting Values of Input Variables in the Analysis of Output from a Computer Code." *Technometrics*, 21(2), 239-245.
   DOI: [10.1080/00401706.1979.10489755](https://www.tandfonline.com/doi/abs/10.1080/00401706.1979.10489755)

2. **@RISK Reproducibility Documentation**
   Palisade/Lumivero. "Reproducibility." @RISK Help Documentation, Version 8.
   URL: [help.palisade.com/v8/en/@RISK/Simulation-Process/Reproducibility.htm](https://help.palisade.com/v8/en/@RISK/Simulation-Process/Reproducibility.htm)

3. **Oracle Crystal Ball Documentation**
   Oracle. "Monte Carlo Simulation and Crystal Ball." Oracle Crystal Ball User's Guide.
   URL: [docs.oracle.com/cd/E57185_01/CYBUG/ch02s01s02.html](https://docs.oracle.com/cd/E57185_01/CYBUG/ch02s01s02.html)

4. **Related ADRs**
   - ADR-016: Monte Carlo Architecture
