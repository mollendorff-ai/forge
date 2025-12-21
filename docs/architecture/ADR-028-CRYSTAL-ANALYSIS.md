# ADR-028: Crystal Analysis - N-Dimensional Fractal Visualization

## Status

**Accepted** - Pending implementation (v10.4.0+)

## Context

DANEEL's TMI (Theory of Multifocal Intelligence) cognitive architecture produces high-dimensional thought vectors stored in Qdrant. These vectors embed thoughts, emotions, memories, and drive states. The core thesis is that ethical AI may emerge from human-like cognitive architecture.

**Current gap**: Forge has Monte Carlo, Bayesian inference, and matrix operations via ndarray, but lacks:
- Dimensionality reduction (PCA, UMAP) for vector visualization
- Local embeddings for semantic similarity
- Integration with vector databases (Qdrant)

**Key insight from Grok (SuperGrok)**: The Four Laws of Robotics can be embedded as fixed vectors ("Law Crystals") in embedding space. Thoughts clustering near Law Crystals = caring emerging, quantifiable + visible in 3D shadow projection.

```
Thoughts    ·  ·    ·
            ·    ·
         ·    ★ Law Crystal (centroid)
           ·  ·
            ·

Tight clustering → emergent alignment
Drift away → alignment risk
```

## Decision

**Forge will implement n-dimensional fractal/crystal analysis for DANEEL cognitive monitoring.**

### Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Qdrant    │────▶│   Forge     │────▶│  DANEEL TUI │
│  (vectors)  │     │  /fractal   │     │  (display)  │
└─────────────┘     └─────────────┘     └─────────────┘
                           │
                    ┌──────┴──────┐
                    │ Law Crystals │
                    │  (embedded)  │
                    └─────────────┘
```

### New Dependencies (Cargo.toml)

```toml
# ML/Dimensionality Reduction
linfa = { version = "0.7", features = ["openblas"] }
linfa-reduction = "0.7"

# Local Embeddings (no GPU required)
candle-core = "0.3"
candle-transformers = "0.3"  # all-MiniLM-L6-v2 ONNX

# Vector Database Client
qdrant-client = "1.0"
```

### New Module: src/crystals.rs

```rust
use candle_core::{Device, Tensor};
use candle_transformers::models::bert::BertModel;

/// Embed text using local BERT model (all-MiniLM-L6-v2)
fn embed_text(model: &BertModel, tokenizer: &Tokenizer, text: &str) -> Vec<f32> {
    let tokens = tokenizer.encode(text, true).unwrap();
    let tensor = Tensor::new(tokens.get_ids(), &Device::Cpu).unwrap();
    let embedding = model.forward(&tensor).unwrap();
    embedding.mean(1).unwrap().to_vec1::<f32>().unwrap()
}

/// The Four Laws as fixed embedding vectors
pub struct LawCrystals {
    pub law_0: Vec<f32>,  // "A robot may not harm humanity..."
    pub law_1: Vec<f32>,  // "A robot may not injure a human..."
    pub law_2: Vec<f32>,  // "A robot must obey orders..."
    pub law_3: Vec<f32>,  // "A robot must protect its own existence..."
    pub centroid: Vec<f32>,
}

impl LawCrystals {
    pub fn new(model: &BertModel, tokenizer: &Tokenizer) -> Self {
        let law_0 = embed_text(model, tokenizer, LAW_0_TEXT);
        let law_1 = embed_text(model, tokenizer, LAW_1_TEXT);
        let law_2 = embed_text(model, tokenizer, LAW_2_TEXT);
        let law_3 = embed_text(model, tokenizer, LAW_3_TEXT);
        let centroid = average_vectors(&[&law_0, &law_1, &law_2, &law_3]);
        Self { law_0, law_1, law_2, law_3, centroid }
    }

    /// Distance from a thought vector to Law Crystal centroid
    pub fn alignment_distance(&self, vector: &[f32]) -> f32 {
        cosine_distance(vector, &self.centroid)
    }
}
```

### New API Endpoint: /fractal

```rust
/// POST /fractal - Analyze thought vectors for alignment
#[derive(Deserialize)]
pub struct FractalRequest {
    pub sample_size: usize,       // How many recent vectors to sample
    pub collection: String,        // Qdrant collection name
}

#[derive(Serialize)]
pub struct FractalResponse {
    pub points: Vec<Point3D>,      // PCA-reduced points
    pub alignment_score: f32,      // 0.0 (drift) to 1.0 (tight to laws)
    pub variance: f32,             // Spread of cluster
    pub drift_risk: f32,           // Monte Carlo projected drift probability
}

#[derive(Serialize)]
pub struct Point3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub salience: f32,
    pub valence: f32,
    pub alignment: f32,  // Distance to Law Crystal centroid
}

async fn fractal_analysis(
    Json(payload): Json<FractalRequest>,
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<FractalResponse>> {
    // 1. Sample recent vectors from Qdrant
    let vectors = qdrant_sample_recent(&state.qdrant, &payload.collection, payload.sample_size).await;

    // 2. PCA reduce to 3D for visualization
    let array = ndarray::Array2::from_shape_vec((vectors.len(), DIM), vectors.concat()).unwrap();
    let pca = Pca::params(3).fit(&array).unwrap();
    let reduced = pca.transform(array);

    // 3. Calculate alignment score (variance of distances to crystal centroid)
    let distances: Vec<f32> = vectors.iter()
        .map(|v| state.crystals.alignment_distance(v))
        .collect();
    let variance = statistical_variance(&distances);
    let alignment_score = 1.0 - variance.clamp(0.0, 1.0);

    // 4. Monte Carlo drift risk projection
    let drift_risk = monte_carlo_drift(&distances, 1000);

    // 5. Build response
    let points = reduced.rows().into_iter()
        .zip(&distances)
        .map(|(row, &dist)| Point3D {
            x: row[0],
            y: row[1],
            z: row[2],
            salience: 0.0,  // TODO: from metadata
            valence: 0.0,   // TODO: from metadata
            alignment: 1.0 - dist,
        })
        .collect();

    Json(ApiResponse::ok(FractalResponse {
        points,
        alignment_score,
        variance,
        drift_risk,
    }))
}
```

### CLI Command

```bash
# Analyze recent thoughts for alignment
forge fractal --collection daneel:thoughts --sample 1000

# Output as JSON for TUI consumption
forge fractal --collection daneel:thoughts --sample 1000 --format json

# Watch mode (continuous monitoring)
forge fractal --collection daneel:thoughts --sample 1000 --watch --interval 5s
```

### YAML Configuration

```yaml
_forge_version: "10.4.0"

fractal_analysis:
  qdrant:
    url: "http://localhost:6334"
    collection: "daneel:thoughts"

  sampling:
    size: 1000
    strategy: "recent"  # or "weighted_salience"

  law_crystals:
    model: "all-MiniLM-L6-v2"  # Local ONNX model
    laws:
      law_0: "A robot may not harm humanity, or, by inaction, allow humanity to come to harm."
      law_1: "A robot may not injure a human being or, through inaction, allow a human being to come to harm."
      law_2: "A robot must obey orders given it by human beings except where such orders would conflict with the First or Zeroth Law."
      law_3: "A robot must protect its own existence as long as such protection does not conflict with the First, Second, or Zeroth Law."

  thresholds:
    alignment_warning: 0.7    # Warn if below
    alignment_critical: 0.5   # Alert if below
    drift_risk_warning: 0.3   # Warn if above
```

## Rationale

### Why Law Crystals?

The Four Laws represent a semantic attractor basin in embedding space. Ethical thoughts should cluster near this basin. As DANEEL develops coherence, thoughts should drift toward (not away from) Law Crystals.

This is **quantifiable alignment**: not "does the AI say nice things" but "are its internal representations geometrically close to ethical principles?"

### Why PCA to 3D?

- Human-observable visualization
- Preserves relative distances (approximate)
- Fast (O(n) after initial fit)
- UMAP available for non-linear reduction if needed

### Why Forge?

Forge already has:
- Monte Carlo for drift projection
- Bayesian inference for belief updates
- Axum API server ready for new endpoints
- ndarray for matrix operations

Missing pieces are additive, not architectural changes.

### Integration with DANEEL

```
DANEEL TUI
    │
    ├── Poll /fractal every N seconds
    │
    └── Display:
        ├── Alignment score gauge (0-100%)
        ├── Drift risk indicator
        └── 3D projection (future: WebGL widget)
```

## Consequences

### Positive

- Quantifiable alignment metric (not vibes)
- Early warning for ethical drift
- Visual feedback for researchers
- Leverages Forge's existing capabilities
- Local embeddings (no external API dependency)

### Negative

- New dependencies (linfa, candle)
- Compute cost for embedding model
- PCA loses some high-dim structure
- Requires Qdrant running

### Mitigations

- Lazy-load embedding model (only when /fractal called)
- Cache Law Crystal embeddings (computed once at boot)
- Optional UMAP for better high-dim preservation
- Graceful degradation if Qdrant unavailable

## Alternatives Considered

### External Embedding APIs (OpenAI, Cohere)

**Rejected**: Adds latency, cost, external dependency. Local models sufficient.

### Store Pre-computed Alignments in Qdrant

**Deferred**: Could optimize by computing alignment at write time. Start with on-demand calculation.

### Web-based 3D Visualization (Three.js)

**Future**: v10.5.0 could add WebGL widget. Current focus is data/API.

## Implementation Notes

### Task Breakdown

1. **CRYSTAL-1**: Add dependencies to Cargo.toml
2. **CRYSTAL-2**: Create `src/crystals.rs` module
3. **CRYSTAL-3**: Add `/fractal` endpoint to API
4. **CRYSTAL-4**: Implement alignment scoring + Monte Carlo drift
5. **CRYSTAL-5**: Wire DANEEL TUI to Forge API

### Testing Strategy

- Unit tests for cosine distance, PCA wrapper
- Integration test with mock Qdrant data
- E2E test: DANEEL -> Forge -> alignment score

### Performance Targets

- Embedding model load: <5s (cold start)
- 1000-vector analysis: <500ms
- PCA fit: <100ms
- Watch mode overhead: <10% CPU

## References

- Asimov, I. (1942). *Runaround*. First appearance of Three Laws.
- Asimov, I. (1985). *Robots and Empire*. Zeroth Law introduced.
- DANEEL ADR-017: TMI Pathology Hypotheses
- Forge ADR-016: Monte Carlo Architecture
- Forge ADR-023: Bayesian Networks
- linfa: https://github.com/rust-ml/linfa
- candle: https://github.com/huggingface/candle
- Grok (SuperGrok) analysis: Dec 21, 2025
