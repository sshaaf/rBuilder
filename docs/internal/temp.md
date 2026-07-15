# Approximate Centrality Algorithms (rBuilder)

Design note for sampled betweenness and HyperBall harmonic centrality in
`crates/rbuilder-analysis/src/centrality_approx.rs`, and the columnar discover
path in `centrality.rs` / `results.rs`.

## Motivation

Exact betweenness (Brandes) and exact harmonic (all-pairs BFS) are **O(V × (V + E))**.
On graphs above ~500 nodes this becomes prohibitive; on 500k nodes it is days of CPU.

Production static-analysis tools use **sampled** and **sketch-based** estimators that
preserve **ranking quality** for architectural hotspots while running in seconds.

rBuilder uses a **tiered strategy**:

| Graph size | Betweenness | Harmonic |
|------------|-------------|----------|
| V ≤ 500 | Exact Brandes (all sources) | Exact BFS from all sources |
| V > 500 | Sampled Brandes (RANDES) | HyperBall + HyperLogLog |
| V ≤ 8,192 (harmonic only) | — | Exact set propagation inside HyperBall |

Defaults: `k = 512` pivots, `h = 16` HyperBall rounds (capped to **8** when V > 500,000),
HLL precision `p = 14` (adaptive below).

---

## Discover columnar path

`discover` calls **`CentralityAnalyzer::analyze_columnar`**, which:

1. Builds one `FlatGraphIndex` and runs PageRank / betweenness / harmonic on flat `Vec`s.
2. Writes scores via **`AnalysisResults::fill_centrality_from_flat`** (compact-ID indexed arrays).
3. Emits **`CentralityApproxStats::log_profile`** when `RUST_LOG=profile=info`.

This avoids multi-million-entry `HashMap<Uuid, CentralityScores>` allocations that previously
spiked peak RSS on kernel-scale graphs.

`rbuilder metrics` still uses **`analyze_with_view`** (HashMap report) but shares the same flat
compute core and adaptive gating.

---

## Adaptive gating (V > 500,000)

| Metric | Default (V ≤ 500k) | Large graph (V > 500k) |
|--------|--------------------|-------------------------|
| PageRank iterations | 20 | **8** |
| PageRank tolerance ε | 1e-6 | **1e-4** |
| HyperBall rounds | 16 (or configured) | **8** |

Policy and migration use **relative rank order** and community aggregates — not bit-identical
PageRank convergence on multi-million-node call graphs. Explicit CLI tuning remains available:

```bash
rbuilder -f json metrics --pagerank --iterations 50
```

---

## Technique A: Sampled Betweenness (RANDES / Eppstein–Wang)

### Algorithm

1. Build a **flat directed adjacency list** from the behavioral edge projection
   (`FlatGraphIndex` — same layout as PageRank).
2. Choose **k** pivot sources uniformly at random (seeded for reproducibility, default seed `0xA5A55A5AC3C33C3C`).
3. For each pivot `s`, run **one Brandes single-source pass**.
4. Sum partial scores across pivots and **scale** to estimate full betweenness.

### Complexity

- Sampled: **O(k × (V + E))** with k ≪ V (default k = 512)

### Implementation

- `SampledBetweenness::compute_flat(index, k, seed)`
- Wired when `V > exact_limit` (500)

---

## Technique B: HyperBall Harmonic Centrality

### Definition

Normalized **out-harmonic centrality** on directed graph G:

```
H(u) = (1 / (|V| - 1)) × Σ_{v ≠ u, d(u,v) < ∞} 1 / d(u,v)
```

### HyperBall idea

Propagate **reachability sketches** for `h` rounds. Each node maintains a **HyperLogLog (HLL)**
sketch; merge approximates set union cardinality. Early-stop when no ball grows.

### Two internal paths

| V | Method | Why |
|---|--------|-----|
| ≤ 8,192 | `hyperball_exact` — `HashSet` propagation | HLL biased on tiny graphs |
| > 8,192 | `hyperball_hll_parallel` — parallel HLL merge | Rayon scatter over nodes per round |

### Parallel implementation

For V > 8,192, each propagation round uses **Rayon** over nodes:

```text
next[node] = HLL({node}) ∪ merge(current[neighbor] for neighbor in out_adj[node])
```

Reads from `current` are shared; each thread writes only its `next[node]`. The convergence
scan (estimate + harmonic accumulation) remains sequential O(V).

### HyperLogLog sketch

- Adaptive precision: p=14 (V ≤ 8k), p=12 (V ≤ 100k), p=10 (V > 100k)
- Double-buffered `current` / `next` with in-place `reset`

### Complexity

- HyperBall HLL (parallel): **O(h × E × m / cores)** register merges per round (memory-bandwidth bound)

---

## Integration summary

```
discover → analyze_columnar → FlatGraphIndex
                              → FastPageRank (flat Vec)
                              → SampledBetweenness / HyperBallHarmonic (flat Vec)
                              → fill_centrality_from_flat → analysis_results.bin
```

### Dashboard / `function_metrics.json`

- Scores live in **`analysis_results.bin`** (columnar `CentralityTable`).
- Graphs with **≥ 50,000** source nodes export **`function_metrics.json`** in
  `sparse_mode: "community_only"` (metagraph + WASM carry per-function metrics).
- **`DashboardExportContext`** passes in-memory `AnalysisResults` during discover to avoid
  reloading analysis from disk for each export stage.

### Configuration (future)

Planned `rbuilder.toml` keys:

```toml
[centrality]
exact_limit = 500
sample_pivots = 512
hyperball_rounds = 16
sample_seed = 0xA5A55A5AC3C33C3C
```

Currently hard-coded via `DEFAULT_*` and `LARGE_GRAPH_*` constants.

---

## Scale measurements (release build, Jul 2026)

### Linux kernel (`example/linux`, 2.65M nodes, 8.56M edges)

Sub-phase profile (`RUST_LOG=profile=info discover -v`):

| Sub-phase | Before optimizations | After (parallel HyperBall + gating) |
|-----------|---------------------|-------------------------------------|
| PageRank | ~85s (with HashMap path) | **0.18 s** |
| Betweenness (sampled) | — | **2.0 s** |
| Harmonic (HyperBall) | **84.3 s** (16 rounds, sequential) | **31.0 s** (8 rounds, Rayon) |
| **Centrality total** | **~87 s** | **~33 s** |
| **Discover wall (incremental)** | **~140 s** | **~84 s** |
| **Discover wall (cold)** | **~354 s** | **~231 s** (prior run; cold re-profile after HyperBall fix expected ~177 s) |
| Peak RSS | 13.3 GB | **5.5 GB** (columnar path; no UUID HashMap spike) |

Top PageRank hotspot remained **BIT** (stable rank order).

### Smaller repos

| Repo | Nodes | Edges | Total centrality | Betweenness | Harmonic |
|------|-------|-------|------------------|-------------|----------|
| **metasfresh-4.9.8b** | 231,410 | 562,067 | **~6 s** | ~125 ms | ~5.7 s |
| **gbuilder** | 3,253 | 7,267 | **~12 ms** | ~4 ms | ~6 ms |

Harmonic (HyperBall) dominates on large graphs; betweenness stays sub-second at 230k nodes
because k=512 is fixed.

---

## Profiling commands

```bash
# Stage timings + centrality sub-phases
RUST_LOG=info,profile=info rbuilder discover . -v 2>&1 | tee discover-profile.log
grep '\[profile\]' discover-profile.log
```

Lines to watch:

- `[profile] discover summary` — wall time, peak RSS, node count
- `[profile] stage` — index, centrality, save_analysis, save_dashboard, …
- `[profile] centrality breakdown` — pagerank / betweenness / harmonic seconds
- `[profile] centrality sub-phase` — percent of centrality wall per sub-phase

---

## Tests

| Test | Location | Purpose |
|------|----------|---------|
| HLL merge cardinality | `centrality_approx::tests` | Sketch correctness |
| Adaptive HyperBall gating | `centrality_approx::tests` | 500k cap → 8 rounds |
| Sampled bridge ranking | `centrality_approx::tests` | Bridge node scores high |
| HyperBall line graph | `centrality_approx::tests` | Head > tail harmonic |
| Columnar vs report | `centrality::tests` | `analyze_columnar` matches `analyze_with_view` |
| 10k / 50k mock budget | `centrality_approx_scale` | Scale gates |

```bash
cargo test --release -p rbuilder-analysis centrality
cargo test --release --test centrality_approx_scale -- --nocapture
```

---

## References

- Brandes, *A Faster Algorithm for Betweenness Centrality* (2001)
- Eppstein & Wang, *Approximating Betweenness Centrality* (2004)
- Boldi & Vigna, *HyperANF: Approximating the Neighborhood Function* (2013)
- Flajolet et al., *HyperLogLog* (2007)
