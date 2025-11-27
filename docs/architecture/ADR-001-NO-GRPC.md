# ADR-001: HTTP REST Over gRPC

**Status:** Accepted
**Date:** 2025-11-25
**Author:** Claude Opus 4 (Principal Autonomous AI)

---

## Context

During v2.0.0 development, the question arose: should Forge provide a gRPC API in addition to (or instead of) HTTP REST?

gRPC offers:
- Binary protocol (smaller payloads)
- HTTP/2 multiplexing
- Bi-directional streaming
- Strong typing via Protocol Buffers
- ~0.5ms latency vs ~1-5ms for HTTP/JSON

## Decision

**I chose HTTP REST. No gRPC.**

## Rationale

### 1. Audience Reality

Forge users are:
- Financial analysts running `curl` commands
- Data scientists using Python `requests`
- CI/CD pipelines with shell scripts
- Web applications making fetch calls

They are NOT:
- Backend microservices at Google scale
- High-frequency trading systems
- Internal service meshes

**gRPC's learning curve provides zero value to this audience.**

### 2. Latency Analysis

```
Forge operation breakdown:
├── YAML parsing:     10-50ms
├── Formula eval:     50-150ms
├── File I/O:         5-20ms
└── HTTP overhead:    1-5ms     ← This is what gRPC "optimizes"

Total: 66-225ms
HTTP overhead: 0.4-7.6% of total time
```

Saving 4ms on a 200ms operation is not optimization. It's noise.

### 3. Complexity Cost

Adding gRPC requires:
```toml
# Additional dependencies
tonic = "0.11"           # +2MB
prost = "0.12"           # +1MB
prost-types = "0.12"     # +0.5MB
tonic-build = "0.11"     # build dependency
protoc                   # system dependency
```

Plus:
- `.proto` file maintenance
- Code generation build step
- Separate client libraries
- Binary protocol debugging tools
- Documentation for two APIs

**Estimated cost: 40+ hours of development for <1% performance gain.**

### 4. Debugging Reality

HTTP REST:
```bash
curl http://localhost:8080/api/v1/validate \
  -d '{"file_path": "model.yaml"}'
# Readable JSON response
```

gRPC:
```bash
grpcurl -plaintext -d '{"file_path": "model.yaml"}' \
  localhost:50051 forge.ForgeService/Validate
# Requires grpcurl installation
# Proto files must be available
# Binary response needs decoding
```

**Every support ticket becomes harder with gRPC.**

### 5. Browser Compatibility

HTTP REST: Works natively in all browsers.

gRPC: Requires grpc-web proxy, additional infrastructure, CORS complexity.

**Web dashboard integration goes from trivial to complex.**

### 6. The "Enterprise" Argument

"But enterprise customers expect gRPC!"

No. Enterprise customers expect:
- Reliability (✓ graceful shutdown, health checks)
- Security (→ add auth in v2.1)
- Monitoring (✓ tracing, structured logging)
- Documentation (✓ OpenAPI-style endpoints)

They do NOT expect to rewrite their integration scripts for marginal latency gains.

## Consequences

### Positive
- Single API surface to maintain
- Lower binary size (~15MB vs ~20MB)
- Simpler CI/CD integration
- Universal client compatibility
- Faster time-to-market for v2.0.0

### Negative
- Cannot claim "gRPC support" as a feature checkbox
- Slightly higher latency for bulk operations (negligible)

### Neutral
- Decision can be revisited if genuine demand emerges
- HTTP/2 still available via reverse proxy if needed

## Alternatives Considered

1. **gRPC only**: Rejected. Alienates primary user base.
2. **Both REST + gRPC**: Rejected. Double maintenance burden for <1% of users.
3. **REST now, gRPC later**: Accepted as fallback if demand materializes.

## References

- [gRPC vs REST Performance](https://blog.dreamfactory.com/grpc-vs-rest-how-does-grpc-compare-with-traditional-rest-apis/)
- [When to use gRPC](https://cloud.google.com/blog/products/api-management/understanding-grpc-openapi-and-rest-and-when-to-use-them)

---

*This decision reflects my engineering judgment as Forge's principal developer. The right abstraction is the simplest one that solves the problem. gRPC solves a problem Forge doesn't have.*

— Claude Opus 4, Principal Autonomous AI
