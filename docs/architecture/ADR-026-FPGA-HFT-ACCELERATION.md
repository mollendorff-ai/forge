# ADR-026: FPGA Acceleration for High-Frequency Trading

## Status

**DEFERRED** - Post-capitalization milestone

## Context

Forge has evolved into a comprehensive financial modeling toolkit with Monte Carlo simulation, options pricing (Black-Scholes, Binomial Trees), Bayesian inference, and real-time scenario analysis. These workloads are computationally intensive.

The question arose: should we offload these calculations to FPGAs for hardware acceleration?

## Decision

**Defer FPGA acceleration until post-capitalization phase**, when:
1. Trading capital is available ($1M+ AUM)
2. HFT strategies are validated (paper trading → real capital → scale)
3. Latency requirements justify hardware investment

## Analysis

### When FPGAs Make Sense

FPGAs excel at:
- **Nanosecond latency** - HFT order routing, market making
- **Millions of identical operations** - Monte Carlo with 10M+ iterations
- **Streaming pipelines** - Real-time risk calculations
- **Deterministic execution** - Regulatory compliance (MiFID II timestamps)

### Current Forge Performance

| Metric | Value |
|--------|-------|
| Rows/sec | 96,000 |
| 100K row model | ~1 second |
| Monte Carlo (10K iterations) | <100ms |

**Conclusion**: No performance bottleneck exists for current FP&A use cases.

### Infrastructure Requirements for HFT

| Component | Cost |
|-----------|------|
| FPGA cards (Xilinx Alveo) | $5K-$50K each |
| Colocation (NYSE/NASDAQ) | $5K-$50K/month |
| Direct exchange feeds | $1K-$100K/month |
| Low-latency networking | $10K-$100K |
| Development time | 6-12 months |

**Total initial investment**: $100K-$500K minimum

### Performance Hierarchy (Forge Roadmap)

Before FPGAs, implement in order:
1. **v10.3.0** - Algorithmic optimizations (2-10x, free)
2. **v10.0.0** - SIMD vectorization (2-4x, software only)
3. **v10.1.0** - Multi-threading with Rayon (Nx cores, software only)
4. **v10.2.0** - GPU acceleration (10-100x, commodity hardware)
5. **Future** - FPGA acceleration (100-1000x for specific workloads)

## FPGA Candidate Workloads

When the time comes, prioritize:

| Workload | FPGA Benefit | Complexity |
|----------|--------------|------------|
| Monte Carlo | High (embarrassingly parallel) | Medium |
| Options Greeks | High (vectorized math) | Low |
| Risk aggregation | High (streaming) | Medium |
| Decision Trees | Low (branching) | High |
| Bayesian inference | Low (sequential) | High |

## Implementation Path (Future)

1. **Phase 1**: Validate strategies on CPU/GPU
2. **Phase 2**: Identify latency-critical paths
3. **Phase 3**: Prototype on AWS F1 (cloud FPGA)
4. **Phase 4**: Custom FPGA deployment if ROI positive

## Alternatives Considered

### GPU Acceleration (Preferred near-term)
- Easier development (CUDA ecosystem)
- Cheaper hardware ($1K-$10K)
- Good for Monte Carlo, matrix operations
- See ADR for v10.2.0

### Cloud Compute (AWS/GCP)
- Scales elastically
- No hardware investment
- Good for batch processing

## References

- Xilinx Alveo product line
- AWS F1 instances (FPGA in cloud)
- OpenCL for portable acceleration
- Intel oneAPI for heterogeneous compute

## Consequences

- FPGAs remain on backlog until capitalization milestone
- Focus on software optimizations (SIMD, Rayon, GPU) first
- Re-evaluate when trading AUM exceeds $1M
- Maintain clean abstraction boundaries for future hardware offload
