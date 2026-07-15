# rbuilder-analysis Architecture

The `rbuilder-analysis` crate bridges the code knowledge graph (`rbuilder-graph`) with static-analysis algorithms. It organizes graph usage into three tiers.

## Tier 1: Repository graph (source of truth)

- **`MemoryBackend`** — in-memory node/edge store with typed topology
- **Mmap snapshots** — `PreparedGraphSnapshot`, `MmappedGraphSnapshot` for zero-copy reads

Nodes represent symbols (functions, types, modules). Edges carry `EdgeType` semantics (`Calls`, `Uses`, `Contains`, `References`, etc.).

## Tier 2: Projected views (repository-level analysis)

| Type | Module | Purpose |
|------|--------|---------|
| `PetGraphView` | `graph_utils` | `petgraph` DiGraph + UnGraph with UUID bi-maps; edge weights are `EdgeType` |
| `CallGraph` | `callgraph` | u32-indexed call-only adjacency for fast traversal |
| `FlatGraphIndex` | `centrality` | Contiguous `usize` edge list for numeric algorithms |

**Convention:** Build `PetGraphView` once per analysis pass and pass `&PetGraphView` to centrality, community, dependency, and blast-radius analyzers.

### Repository-level analyses

| Analysis | Primary graph | Algorithm |
|----------|---------------|-----------|
| Centrality | `PetGraphView` → `FlatGraphIndex` | PageRank, sampled Brandes betweenness, HyperBall harmonic |
| Community | `PetGraphView` (undirected filter) | Label propagation + modularity ([naming note](design/graph-metrics-design.md#31-community-detection-naming)) |
| Blast radius (engine) | Call-only DiGraph | Kosaraju SCC → on-demand reachability (flat graphs) or bitset rows |
| Blast radius (analyzer) | `PetGraphView` Calls filter | Reverse BFS |
| Dependencies | `PetGraphView` directed | Kosaraju SCC, reverse BFS impact |
| Complexity | Backend node properties | Aggregation |
| Migration | Community graph | Weighted topological sort |

## Tier 3: Intra-procedural graphs

| Type | Module | Purpose |
|------|--------|---------|
| `ControlFlowGraph` | `cfg`, `cfg_builder` | Per-function CFG via tree-sitter |
| `ProgramDependenceGraph` | `pdg` | Data + control dependencies |
| `DominatorTree` | `dominance` | Immediate dominators and frontiers |

### Pipeline

```
tree-sitter AST → CFG → DominatorTree + ReachingDefs → PDG → Slicing / Taint
```

`InterproceduralCFG` stitches per-function CFGs with `CallGraph` for cross-function slicing.

## Traversal depth

Graph BFS traversals (blast radius analyzer, dependency impact) share `TraversalConfig` with default depth **10** (`DEFAULT_TRAVERSAL_DEPTH` in `graph_utils`). Use `TraversalConfig::unlimited()` for full transitive closure; prefer `BlastRadiusEngine` for large graphs.

## Caching and persistence

- **`FlowCache`** / **`CfgPdgArchive`** — per-function CFG/PDG cache
- **`BlastEngineSnapshot`** — persisted SCC reachability bitsets
- **`AnalysisResults`** — columnar metrics decoupled from graph topology (`CentralityTable`, community, blast tables)

### Centrality pipeline (discover)

Discover uses **`CentralityAnalyzer::analyze_columnar`**: flat scores from `FlatGraphIndex` are written directly into `AnalysisResults` without intermediate `HashMap<Uuid, _>` handoffs.

| Graph size | PageRank | Betweenness | Harmonic |
|------------|----------|-------------|----------|
| V ≤ 500 | Exact (20 iter, ε=1e-6) | Exact Brandes | Exact BFS |
| 500 < V ≤ 500,000 | Exact (20 iter) | Sampled Brandes (k=512) | HyperBall (h=16, parallel HLL) |
| V > 500,000 | Gated (8 iter, ε=1e-4) | Sampled Brandes (k=512) | HyperBall (h=8, parallel HLL) |

Constants: `LARGE_GRAPH_PAGERANK_*`, `LARGE_GRAPH_HYPERBALL_*` in `centrality.rs` / `centrality_approx.rs`.

**Profiling:** `discover -v` with `RUST_LOG=profile=info` emits `[profile] stage` and `[profile] centrality sub-phase` lines (PageRank, betweenness, harmonic, columnar fill timings).

See [internal/temp.md](internal/temp.md) for algorithm detail and kernel-scale measurements.

## Further reading

- Crate README: `crates/rbuilder-analysis/README.md`
- OpenSpec design: `openspec/changes/review-rbuilder-analysis/design.md`
- Benchmarks: `cargo bench --bench graph_benchmarks` (workspace root)
