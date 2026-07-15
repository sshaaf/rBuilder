# rBuilder

**A code knowledge graph built for LLM agents — accurate answers, minimal tokens, maximum speed.**

AI coding agents default to reading files sequentially. That burns context, misses structure, and produces confident wrong answers about impact and dependencies. **rBuilder indexes the whole repository once** into a rich graph with pre-computed **reachability**, then serves **compact, deterministic query results** — so agents (and humans) get the right slice of the codebase without loading it into the prompt.

[![CI](https://github.com/sshaaf/rBuilder/actions/workflows/ci.yml/badge.svg)](https://github.com/sshaaf/rBuilder/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Demo

**~65s** dashboard tour (5 s per feature) — discover metrics, GQL, semantic search, graph metrics, CFG/PDG, slicing, blast radius, taint, migration planner, CI policy, and export. Each segment highlights the active tab and its panel.

https://github.com/user-attachments/assets/547bf5d1-2058-4668-b990-35def9c38636


Regenerate: [docs/videos/README.md](docs/videos/README.md)

---

## What the **R** stands for

| **R** | Meaning |
|-------|---------|
| **Rust** | Memory-safe, predictable performance at scale — the foundation for parsing large monorepos without blowing the heap |
| **Reachability** | Pre-computed call reachability (sparse bitsets, not multi‑GB dense matrices) so “what breaks if I change this?” stays sub-second |
| **Rich** code graph | 30+ typed relations — CALLS, IMPORTS, CONTAINS, IMPLEMENTS, and more — not just files and folders |

Together: **rBuilder** is the **reachability builder** — it constructs the graph and the compressed reachability engine agents need for trustworthy structural reasoning.

Algorithm and complexity details: crate READMEs under `crates/rbuilder-analysis/` and [CLI I/O sanity QE](docs/cli-io-sanity-qe.md) for automated perf gates.

## Built for agents

**Goal:** make LLM-assisted development **more accurate** while **using fewer tokens**.

| Without rBuilder | With rBuilder |
|------------------|---------------|
| Agent reads dozens of files to guess dependencies | Agent calls `blast-radius Symbol` → structured impact JSON |
| “What calls this?” requires search + inference | `gql` returns exact graph matches |
| Migration planning from partial context | **Migration planner** — package roadmap, dual ordering, tunable scores, interactive graph |
| Repeated file dumps every turn | One `discover`, then cache-backed queries via CLI or `-f json` |

rBuilder answers **reachability and relation questions deterministically** from the indexed graph. The LLM reasons on **summaries and facts**, not raw repo grep — fewer tokens, less hallucination, faster turns.

**Primary outputs for agents:** `-f json` on `discover`, `gql`, `blast-radius`, `metrics`, and `export`. See **[JSON API](docs/json-api.md)**.

---

## Where most tools stop

Most codebase tools stop at **text search**, **file trees**, or a **shallow call graph**. rBuilder goes further — compiler-grade structure and security analysis, pre-computed at index time, queryable in milliseconds. That is what makes agent answers trustworthy.

| Feature | What it gives you | Design doc |
|---------|-------------------|------------|
| **Semantic search** | **Natural-language and keyword search** over functions — bundled code-daemon embeddings, Hamming retrieval, late fusion with blast/PageRank/sketches | [semantic-search-design.md](docs/design/semantic-search-design.md) |
| **Blast radius** | Pre-computed **reachability** over the call graph — upstream impact, scores, policy gates, sub-second on large repos | [blast-radius-design.md](docs/design/blast-radius-design.md) |
| **Program slicing** | **Backward / forward slice** — only the statements that affect (or are affected by) a line and variable | [program-slicing-design.md](docs/design/program-slicing-design.md) |
| **Taint analysis** | **Source → sink** flows (HTTP params → SQL, shell, render, …) with sanitizer awareness | [taint-analysis-design.md](docs/design/taint-analysis-design.md) |
| **CFG** | **Control-flow graph** per function — branches, loops, executable paths | [cfg-design.md](docs/design/cfg-design.md) |
| **PDG** | **Program dependence graph** — data and control deps between statements; foundation for slice and taint | [pdg-design.md](docs/design/pdg-design.md) |
| **Dominance** | **Dominator trees** and frontiers — the same structures compilers use for advanced analysis | [dominance-design.md](docs/design/dominance-design.md) |
| **GQL** | **Graph query language** over 30+ relation types — inventory, call chains, patterns | [gql-design.md](docs/design/gql-design.md) |
| **Graph metrics** | **PageRank**, **betweenness**, **communities** on the live call graph | [graph-metrics-design.md](docs/design/graph-metrics-design.md) |
| **Migration planner** | **Package-level roadmap** — PageRank + harmonic centrality − blast radius; dependency-aware schedule and priority rank; ForceAtlas2 graph in the dashboard | [migration-planner-design.md](docs/design/migration-planner-design.md) |
| **CI policy checks** | **`check`** — fail builds when blast-radius rules are violated on touched symbols | [ci-policy-checks-design.md](docs/design/ci-policy-checks-design.md) |

All of the above share one index: run [`discover`](docs/Introduction.md#indexing-the-repository-discover) once (use [`discover --cfg`](docs/Introduction.md#indexing-the-repository-discover) or `--all` for full CFG/PDG/taint archives and migration exports). **Semantic search** is opt-in: `rbuilder semantic index` after discover. Explore in the CLI, pipe **JSON** to agents, or open the **[dashboard](docs/Introduction.md#dashboard-visual-exploration)** (including **Search** and **Migration** tabs).

**Deep dive on every feature → [Introduction](docs/Introduction.md) · [Feature designs](docs/design/README.md)**

---

## Speed by design

rBuilder is **async and parallel by design** — discovery walks the tree, parses languages concurrently, and builds analytics on the graph in parallel (Rayon + Tokio throughout the pipeline).

- **Full discovery in seconds** on typical repos (not minutes of ad-hoc agent exploration)
- **Reachability compressed** — enterprise-scale call graphs stored in compact on-disk snapshots, not gigabytes in RAM
- **HTTP `serve`** — dashboard + `/api/query` on port 8080; optional `serve --daemon` socket for blast-radius warm path

Index once → query many times. That is the agent workflow.

```text
  Agent / script / human
           │
           ▼
    rbuilder gql | blast-radius | metrics | export -f json
           │
           ▼
  .rbuilder/  ← graph snapshot + reachability engine + indexes
           ▲
           │
      discover .     ← async parallel index (seconds)
           ▲
           │
     Your repository
```

---

## Code understanding and migrations

Use the features above together for **migration and modernization** work:

- **Migration planner** — package-level graph, tunable scoring presets, dependency-aware schedule vs. priority rank
- **Blast radius** + **metrics** — see fan-in and architectural hotspots before moving a service or framework
- **GQL** + **export** — inventory symbols and ship subgraphs to downstream tools
- **Slice** + **taint** — validate data-flow assumptions agents often get wrong
- **`check`** — enforce blast-radius policy in CI while agents (or humans) land changes

### Migration planner

After `discover --all`, open the dashboard **Migration** tab or export a machine-readable plan for agents and downstream tools:

*Unified view: tune α/β/γ weights and presets, explore the package call graph (community-colored by label propagation — see [graph metrics naming](docs/design/graph-metrics-design.md#31-community-detection-naming)), and paginate through the ordered package table.*

- **Package macro graph** — aggregates functions into path-derived package labels (Java package paths, Rust/C `/src/` modules)
- **Dual ordering** — **scheduled step** (Kahn topological sort, callee before caller) and **priority rank** (score-only)
- **Scoring** — `Priority = α·PageRank + β·Harmonic − γ·Blast`; presets include Hybrid Default, Foundational First, Dense Cluster Extraction, Risk Mitigation
- **CLI export** — every `discover` writes `migration_graph.json` and a default `migration_plan.json` under `.rbuilder/dashboard/`; use `--export-migration-plan` to write a preset-tuned plan (default `.rbuilder/migration_plan.json`, override with `-o`)

```bash
rbuilder discover . --all --export-migration-plan
rbuilder serve   # http://127.0.0.1:8080/ → Migration tab
```

Design → **[Migration planner design](docs/design/migration-planner-design.md)** · Workflow → **[Building a migration plan](docs/building-migration-plan.md)**  
All feature designs → **[docs/design/](docs/design/README.md)**

### Community detection naming

rBuilder does **not** run the Leiden algorithm today. What ships is **label propagation** (Raghavan et al., 2007) with Newman modularity scoring, plus hub stripping and deterministic tie-breaking. Docs/UI still say “Louvain” in places (`louvain_community_id`, migration layout), and `TASK_PLAN.md` lists Leiden as planned but unimplemented.

| Name in repo | What it actually is |
|--------------|---------------------|
| `CommunityDetector` | Label propagation on `Calls` + `Uses` |
| “Louvain” in dashboard/migration | Majority vote of label-propagation ids |
| Leiden (task 2.1.1) | Not implemented |

Full detail → **[Graph metrics — community naming](docs/design/graph-metrics-design.md#31-community-detection-naming)**.

Walkthrough on a real Java repo → **[coolstore example](docs/user-guide.md#3-example-project-coolstore)** (User Guide).

**Research map** — which papers rBuilder implements, which inspire the roadmap, and where to propose changes → **[Further reading](docs/further-reading.md#research-foundations-in-rbuilder)**.

---

## Quick start

**Install** from [GitHub Releases](https://github.com/sshaaf/rBuilder/releases) or build from source:

```bash
git clone https://github.com/sshaaf/rBuilder.git
cd rBuilder
git lfs pull   # bundled code-daemon ONNX weights (~206 MB)
cargo build --release
```

**Discover** (build the graph + reachability caches):

```bash
git clone https://github.com/konveyor-ecosystem/coolstore.git
cd coolstore
rbuilder discover .
# agent-friendly telemetry:
rbuilder -f json discover . | jq '.metrics'
```

**Query** (compact answers instead of file dumps):

```bash
# Graph inventory for the agent
rbuilder -f json gql 'MATCH (n:Function) RETURN n LIMIT 10'

# Impact — critical before the agent edits a symbol
rbuilder -f json blast-radius ShoppingCartService

# Hotspots — where migration/refactor pain concentrates
rbuilder -f json metrics --pagerank --communities

# Package migration roadmap (graph + plan JSON for agents)
rbuilder discover . --all --export-migration-plan
```

Concepts → **[Introduction](docs/Introduction.md)** · Commands → **[User Guide](docs/user-guide.md)**

Example deep-analysis commands (after `discover --cfg`):

```bash
rbuilder inspect MyClass#myMethod          # CFG / PDG / dominance
rbuilder slice src/Foo.java --line 42 --variable x
rbuilder slice src/Foo.java --line 10 --variable req --taint
```

---

## Command reference

Quick links into **[Introduction](docs/Introduction.md)** — see [Where most tools stop](#where-most-tools-stop) for the differentiators.

| Command | Introduction |
|---------|----------------|
| `discover` | [Indexing](docs/Introduction.md#indexing-the-repository-discover) |
| `gql` | [Graph queries](docs/Introduction.md#graph-queries-gql) |
| `blast-radius` | [Blast radius](docs/Introduction.md#blast-radius-change-impact) |
| `slice` | [Program slicing](docs/Introduction.md#program-slicing) · [Taint](docs/Introduction.md#taint-analysis) |
| `inspect` | [CFG, PDG, dominance](docs/Introduction.md#cfg-pdg-and-dominance-deep-structure) |
| `metrics` | [Graph metrics](docs/Introduction.md#graph-metrics-architecture-hotspots) |
| `export` | [Export](docs/Introduction.md#export-and-sharing) |
| `check` | [CI policy](docs/Introduction.md#ci-policy-checks) |
| `serve` | [HTTP server](docs/Introduction.md#http-server-serve) |

**Dashboard** — visual exploration after `discover` (`.rbuilder/dashboard/`). See **[Feature designs](docs/design/README.md)** for per-tab engineering docs.  
**Migration export** — `discover --export-migration-plan` (optional `--migration-preset`, `--migration-order scheduled|priority`).  
**Languages** — nine Tier 1 languages (Rust, Python, Java, Go, TypeScript, JavaScript, C#, C, C++) plus config/IaC plugins. See [Language guide](docs/LANGUAGE_GUIDE.md).

---

## Documentation

| Document | For |
|----------|-----|
| **[Documentation index](docs/README.md)** | Map of all docs by persona |
| **[Introduction](docs/Introduction.md)** | Concepts — graph, reachability, each feature |
| **[User Guide](docs/user-guide.md)** | Install, coolstore, every CLI command |
| **[Dashboard user guide](docs/dashboard-user-guide.md)** | Browser UI tab-by-tab |
| **[AGENTS.md](AGENTS.md)** | **LLM agents** — discover once, query JSON |
| **[Agent recipes](docs/agent-recipes.md)** | Copy-paste automation workflows |
| **[JSON API](docs/json-api.md)** | Parse `-f json` payloads |
| **[HTTP API](docs/http-api.md)** | `rbuilder serve` → `/api/query` |
| **[Policy format](docs/policy-format.md)** | `check` / blast policy JSON |
| **[Language guide](docs/LANGUAGE_GUIDE.md)** | Supported languages and tiers |
| **[Further reading](docs/further-reading.md)** | Research implemented vs inspired |
| **[CLI output schemas](docs/cli-output-schemas.md)** | Exact field tables |
| **[CLI I/O sanity QE](docs/cli-io-sanity-qe.md)** | Subprocess JSON contract and release perf gates |
| **[Feature designs](docs/design/README.md)** | Engineering design docs with dashboard screenshots |
| **[Migration planner design](docs/design/migration-planner-design.md)** | Package graph, scoring, ordering |
| **[Building a migration plan](docs/building-migration-plan.md)** | End-to-end migration workflow |
| **[CONTRIBUTING.md](CONTRIBUTING.md)** | Dev setup and PR expectations |
| **[Releasing](docs/releasing.md)** | Tags and GitHub Releases |

---

## Development

```bash
cargo test
cargo build --release
# See CONTRIBUTING.md for dashboard build and golden-repo checks
```

---

## License

MIT — see [LICENSE](LICENSE).
