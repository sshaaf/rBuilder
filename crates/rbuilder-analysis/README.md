# rbuilder-analysis

Graph analysis algorithms for [rBuilder](https://github.com/sshaaf/rBuilder): centrality, community detection, blast radius, CFG/PDG construction, slicing, and taint analysis.

See [docs/analysis-architecture.md](../../docs/analysis-architecture.md) for the three-tier graph model.

## Module index

| Module | Graph input | Algorithm | Complexity |
|--------|-------------|-----------|------------|
| `graph_utils` | Backend / snapshot | Topology projection | O(V+E) |
| `callgraph` | Backend | u32 adjacency build | O(V+E) |
| `centrality` | `PetGraphView` | PageRank, Brandes / sampled betweenness, columnar discover path | O(k·E) PageRank; approximate BC/Harm |
| `centrality_approx` | `FlatGraphIndex` | Sampled Brandes, parallel HyperBall HLL | Approximate; dominates harmonic on large V |
| `community` | `PetGraphView` | Label propagation + modularity | O(iters·E) |
| `blast_radius_scc` | Call DiGraph | Kosaraju + bitset reachability | O(1) query |
| `blast_radius` | `PetGraphView` | Reverse BFS | O(V+E) |
| `dependency` | `PetGraphView` | Kosaraju SCC, reverse BFS | O(V+E) |
| `complexity` | Backend properties | Aggregation | O(F) |
| `cfg_builder` | tree-sitter AST | CFG construction | O(stmts) |
| `dominance` | CFG | Cooper-Harvey-Kennedy idom | O(n²) worst |
| `dataflow` | CFG + PDG | Reaching definitions | O(n·d) |
| `pdg` | CFG | Data + control dependencies | O(n·d) |
| `slicing` | PDG | Backward BFS slice | O(V+E) |
| `taint` | PDG | Forward taint propagation | O(V+E) |
| `migration` | Community graph | Weighted topo sort | O(V+E) |
| `results` | — | Columnar metric storage | O(1) lookup |

## Community detection naming

rBuilder does **not** run the Leiden algorithm today. What ships is **label propagation** ([Raghavan et al., 2007](https://doi.org/10.1107/S1744309107073516)) with Newman modularity scoring, plus hub stripping and deterministic tie-breaking. Docs/UI still say “Louvain” in places (`louvain_community_id`, migration layout), and [`.github/TASK_PLAN.md`](../../.github/TASK_PLAN.md) lists Leiden as planned but unimplemented.

| Name in repo | What it actually is |
|--------------|---------------------|
| `CommunityDetector` | Label propagation on `Calls` + `Uses` |
| “Louvain” in dashboard/migration | Majority vote of label-propagation ids |
| Leiden (task 2.1.1) | Not implemented |

See also [graph-metrics-design.md](../../docs/design/graph-metrics-design.md#31-community-detection-naming).

## Running tests and benchmarks

```bash
cargo test -p rbuilder-analysis
cargo clippy -p rbuilder-analysis -- -D warnings
cargo bench --bench graph_benchmarks      # PetGraphView + blast radius
cargo bench --bench centrality_benchmarks   # PageRank, betweenness
cargo bench --bench community_benchmarks    # Label propagation
```

## Conventions

- Build `PetGraphView` once per analysis pass; pass references to analyzers.
- Use `TraversalConfig` (default depth 10) for bounded BFS traversals.
- Prefer `BlastRadiusEngine` over `BlastRadiusAnalyzer` for large graphs needing full transitive closure.
- Discover uses **`analyze_columnar`** for centrality; profile with `RUST_LOG=profile=info discover -v`.
- No `unwrap()` in production paths; propagate errors with `rbuilder_error::Result`.
