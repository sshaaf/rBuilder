# Introduction to rBuilder

This document explains **what rBuilder is**, how a **code knowledge graph** works, and what each capability is for — before you run commands. For install steps and copy-paste examples, use the **[User Guide](user-guide.md)**.

**Audience:** architects, team leads, security engineers, and developers who want the concepts first.  
**Hands-on next step:** [User Guide → ecommerce-java example](user-guide.md#3-example-project-ecommerce-java)

---

## Table of contents

1. [What problem does rBuilder solve?](#what-problem-does-rbuilder-solve)
2. [What is a code knowledge graph?](#what-is-a-code-knowledge-graph)
3. [How rBuilder fits together](#how-rbuilder-fits-together)
4. [Indexing the repository (`discover`)](#indexing-the-repository-discover)
5. [Graph queries (GQL)](#graph-queries-gql)
6. [Blast radius (change impact)](#blast-radius-change-impact)
7. [Program slicing](#program-slicing)
8. [Taint analysis](#taint-analysis)
9. [CFG, PDG, and dominance (deep structure)](#cfg-pdg-and-dominance-deep-structure)
10. [Hybrid CPG (mutations and flows)](#hybrid-cpg-mutations-and-flows)
11. [Semantic search (opt-in)](#semantic-search-opt-in)
12. [Graph metrics (architecture hotspots)](#graph-metrics-architecture-hotspots)
13. [Migration planner (package roadmap)](#migration-planner-package-roadmap)
14. [Export and sharing](#export-and-sharing)
15. [CI policy checks](#ci-policy-checks)
16. [HTTP server (`serve`)](#http-server-serve)
17. [Dashboard (visual exploration)](#dashboard-visual-exploration)
18. [Where to go next](#where-to-go-next)

---

## What problem does rBuilder solve?

Modern codebases are too large to hold in your head. When you change a function, you need to know:

- Who calls it upstream?
- Which services or modules depend on it?
- Could this change affect security-sensitive paths?
- Where is complexity concentrated?

Reading files one by one is slow and error-prone. **rBuilder turns your repository into a structured graph** — functions, classes, calls, imports, and more — so you can **ask structural questions** and get answers in seconds instead of hours.

The tool is built in **Rust** for speed and predictable memory use: large enterprise repos (hundreds of thousands of nodes) can be indexed in one pass, with analysis results stored in compact on-disk caches rather than loading everything into RAM.

---

## What is a code knowledge graph?

Think of your codebase as a **map**, not a pile of files.

| Everyday idea | In rBuilder |
|---------------|-------------|
| Places on the map | **Nodes** — functions, classes, files, modules, config keys, … |
| Roads between places | **Edges** — typed **relations** between nodes |
| A travel guide | The **graph** stored under `.rbuilder/` after indexing |

### Relations (edges)

rBuilder records many relation types, for example:

- **CALLS** — one function invokes another  
- **CONTAINS** — a class or file holds a member  
- **IMPORTS** — dependency between compilation units  
- **IMPLEMENTS** / **DEPENDS_ON** — structural coupling  

Together, these form a **rich relation matrix**: you see not only “what exists,” but **how parts connect**.

### Reachability

Many questions are really reachability questions: *“If I change X, what else can be affected along call paths?”*

rBuilder pre-computes **reachability** over the call graph (who can reach whom upstream) and stores it in a **compressed snapshot** (sparse bitsets instead of a multi-gigabyte dense matrix). That is what makes **blast radius** queries fast on large graphs — the “R” in rBuilder aligns with **reachability** and **relations** as first-class ideas.

You do not need graph theory to use the CLI; it helps to know that **indexing builds the map**, and **commands query the map**.

---

## How rBuilder fits together

```text
  Your repo (source files)
           │
           ▼
      discover          ← scan, parse, build graph + caches
           │
           ▼
  .rbuilder/            ← graph snapshot, blast engine, indexes
           │
     ┌─────┴─────┬─────────────┬──────────────┐
     ▼           ▼             ▼              ▼
   gql      blast-radius    slice/inspect   metrics/export
                                              migration plan
```

1. **Once per repo (or after big changes):** run `discover` to build `.rbuilder/`.  
2. **Many times:** run query commands (`gql`, `blast-radius`, …) against that cache.  
3. **Optional:** open the **dashboard** for interactive exploration ([dashboard user guide](dashboard-user-guide.md)), or use **`-f json`** for automation ([JSON API](json-api.md)).

Feature-level engineering designs (with dashboard screenshots): **[design/](design/README.md)**.

---

## Indexing the repository (`discover`)

### Goal

Turn a folder of source code into a **persistent, queryable graph** plus pre-computed analysis (complexity, communities, blast-radius scores, optional CFG/PDG).

### Description

`discover` walks the repository, uses language-aware parsers to extract symbols and relationships, and writes artifacts to `.rbuilder/`. The **primary graph** is a columnar binary snapshot (`graph.snapshot.bin`); GQL and most commands read that via mmap — **not** a SQL database. Also written: a blast-radius engine snapshot, a **SQLite blast-radius lookup cache** (`macro_call_index.db`, `blast-radius` fast path only), and optionally per-function control-flow and taint data when you enable deeper modes (`--with-cfg` or `--with-taint`).

Default discover is tuned for speed. Deeper modes trade time for semantic detail (slicing, taint, inspect overlays).

### Key benefits

- **One command** to prepare the whole repo for all other features  
- **Incremental-friendly** file tracking for faster re-runs after small changes  
- **CI-friendly** telemetry with `-f json` (file counts, nodes, edges, duration)  
- **Optional security scan** (`--with-security`, alias `--security`) and **optional CFG/PDG/taint** (`--with-cfg`, `--with-taint`)
- **Optional migration roadmap** (`--export-migration-hints`, with `--migration-preset` and `--migration-order scheduled|priority`)

### How to run it

→ [User Guide §4 — Index with `discover`](user-guide.md#4-index-with-discover)  
→ Artifacts and caches: **[Graph storage architecture](graph-storage-architecture.md)**

---

## Graph queries (GQL)

### Goal

**Explore and inventory** the codebase using a small graph query language — like SQL for structure, not for table rows.

### Description

**GQL** (graph query language) matches patterns in the graph: find all functions whose name contains `Cart`, list call chains between functions, or count nodes by type. Results can be human-readable text or **JSON** for scripts.

Named **macros** (`all_functions`, `direct_calls`, `call_chain`, `all_communities`) bundle common patterns so you do not rewrite long queries. Communities are an **analysis overlay** (virtual `:Community` + `community_id`) — not membership edges in the topology snapshot. The dashboard **Graph** tab visualizes the same package metagraph that many inventory queries summarize.

### Key benefits

- **Fast orientation** in unfamiliar repos (“how many functions?”, “who calls whom?”, “what are the clusters?”)  
- **Repeatable audits** — same query on every release  
- **Automation** — pipe JSON to `jq` or your own tools  
- **No LLM required** — deterministic answers from the indexed graph

### How to run it

→ [User Guide §6 — Query the graph with GQL](user-guide.md#6-query-the-graph-with-gql) (includes named communities)  
→ Design: **[GQL design](design/gql-design.md)** · **[Community query & naming](design/community-query-and-naming-plan.md)** · HTTP: **[HTTP API](http-api.md)**

---

## Blast radius (change impact)

### Goal

Answer: **“If I change this function or method, what breaks upstream?”** — before you merge the change.

### Description

**Blast radius** walks the **incoming call graph** (callers and transitive callers) from a chosen symbol. It returns an impact **score**, lists of **direct callers** and the wider **impact zone**, and (with JSON) stable **UUIDs** and **canonical names** for automation.

You can cap how far upstream to look (`--depth`), attach **policy files** for governance (e.g. “this change must not cross domain boundaries”), and optionally request **slice hand-offs** for line-level follow-up.

Pre-computed reachability at discover time is what keeps this sub-second on large graphs. A **SQLite blast lookup cache** (`macro_call_index.db`) accelerates repeat queries on uniquely named symbols; the graph itself lives in `graph.snapshot.bin`, not SQL.

### Key benefits

- **Change-risk triage** before code review or release  
- **Refactoring safety** — see fan-in before renaming or deleting APIs  
- **Policy gates** — fail CI when impact crosses forbidden boundaries  
- **Structured JSON** for tickets, bots, and agent workflows

### How to run it

→ [User Guide §7 — Blast radius (change impact)](user-guide.md#7-blast-radius-change-impact)  
→ Design: **[Blast radius design](design/blast-radius-design.md)**

---

## Program slicing

### Goal

Answer: **“Which lines of this function actually affect this variable at this line?”** — backward or forward through data and control dependencies.

### Description

**Slicing** is a precision tool for debugging and review. You point at a **file**, **line**, **variable**, and enclosing **method name** (`--function`); rBuilder computes the **slice** — the minimal set of statements that influence (or are influenced by) that point. This uses control-flow and program-dependence structure inside the function.

Slicing reads source from disk; richer cross-function context is available when the repo was indexed with `discover --with-cfg`. The dashboard **Program Slicing** tab runs the same analysis in the browser with highlighted source.

### Key benefits

- **Narrow focus** during incident response (“what fed this value?”)  
- **Review efficiency** — less noise than reading the whole file  
- **Exportable views** — text summary, or CFG/PDG overlays with Mermaid/Graphviz

### How to run it

→ [User Guide §8 — Program slicing and taint](user-guide.md#8-program-slicing-and-taint) (slice sections)  
→ Design: **[Program slicing design](design/program-slicing-design.md)**

---

## Taint analysis

### Goal

Find **unsafe flows** where untrusted input (sources) may reach dangerous operations (sinks) — e.g. HTTP parameters into SQL, or user input into shell commands.

### Description

**Taint analysis** tracks how data of interest propagates from **sources** (request parameters, files, environment variables, …) to **sinks** (SQL execution, shell, HTML render, …). Flows may be **sanitized** on the path; vulnerable flows are those with no effective sanitizer.

At CLI level, `slice --taint` gives a quick per-function check. Full-repo taint summaries are produced when you run `discover --with-cfg` or `--with-taint` and appear in the dashboard **Taint Analysis** tab and exported JSON indexes.

### Key benefits

- **Security review** without manual path tracing on every endpoint  
- **Severity hints** from source/sink pairing  
- **Integration** with discover pipeline for batch reporting across functions

### How to run it

→ [User Guide §8 — Program slicing and taint](user-guide.md#8-program-slicing-and-taint) (taint sections)  
→ Deeper index: `discover . --with-cfg` (alias `--cfg`) or `discover . --with-cfg --with-security --with-taint` ([User Guide §4](user-guide.md#4-index-with-discover))  
→ Design: **[Taint analysis design](design/taint-analysis-design.md)**

---

## CFG, PDG, and dominance (deep structure)

### Goal

Inspect **how code executes inside a single function** — branches, loops, data dependencies between statements, and dominance structure used by compilers and advanced analyses.

### Description

| Concept | Meaning |
|---------|---------|
| **CFG** (control-flow graph) | Blocks and branches: what can run after what |
| **PDG** (program dependence graph) | Data and control edges between statements |
| **Dominance** | Which blocks must execute before others; dominance frontiers for SSA-style reasoning |

The **`inspect`** command dumps these layers for a named function. **`discover --with-cfg`** (alias `--cfg`) must have run so the archive contains CFG/PDG data for indexed symbols.

In the dashboard: **CFG / PDG Analysis** tab (control-flow graph + idom table), **Dataflow** tab (PDG and dominator-tree views).

### Key benefits

- **Compiler-minded debugging** without leaving the repo tool chain  
- **Foundation** for slice, taint, and dataflow features  
- **Diagram export** (Mermaid, Graphviz) for docs and reviews

### How to run it

→ [User Guide §9 — Inspect CFG / PDG / dominance](user-guide.md#9-inspect-cfg--pdg--dominance)  
→ Design: **[CFG](design/cfg-design.md)** · **[PDG](design/pdg-design.md)** · **[Dominance](design/dominance-design.md)**

---

## Hybrid CPG (mutations and flows)

**Bridge** the repo-level call graph with per-function CFG/PDG so agents can ask Joern-style questions — “who mutates this type outside constructors?” — without reading whole files.

After `discover --with-cfg`, the **`cpg`** commands expose:

- **`cpg status`** — is the L_proc archive ready? how many field writes indexed?
- **`cpg mutations --type T --exclude-ctors`** — typed field writes (CoolStore demo: `ShoppingCart` on ecommerce fixtures)
- **`cpg flows`** — forward/backward data dependence from a variable at a line
- **`cpg calls` / `cpg function`** — CALL neighborhood bridged to L_proc

The in-tree fixtures keep **`/api/*`** and add CoolStore-compatible **`/services/*`** with `ShoppingCartService.priceShoppingCart` as a clear mutation/pricing site.

**Benefits**

- **DTO / record safety** — empty mutations ⇒ no typed non-ctor writes found  
- **Same CLI for agents** — prefer `-f json cpg …` over ad-hoc multi-tool glue  
- **Language honesty** — typed recovery varies (C uses `shopping_cart_t`; JS may need `--include-unresolved`)

→ [User Guide §10 — Hybrid CPG](user-guide.md#10-hybrid-cpg-cpg)  
→ [Agent recipes — Recipe 11](agent-recipes.md) · [hybrid-cpg-plan.md](design/hybrid-cpg-plan.md)

---

## Semantic search (opt-in)

### Goal

Answer: **“Which functions match this natural-language or keyword intent?”** — without grepping the whole tree into an LLM context.

### Description

**Semantic search** is a separate opt-in index over function embeddings. After `discover`, run `semantic index`, then `semantic query` (or the dashboard **Search** tab via `serve`). Default embedder is **code-daemon** (ONNX weights via Git LFS); offline alternatives are `--embedder vocab` or `--embedder hash`. Retrieval uses Hamming distance over packed bit vectors, with optional late fusion against blast / PageRank / sketches.

It does **not** replace GQL or blast-radius — use it when you know the *intent* but not the exact symbol name.

### Key benefits

- **Intent → symbols** for agents and humans  
- **Offline modes** when ONNX weights are unavailable  
- **Same JSON contract** as other `-f json` commands (`schema_version` on stdout)

### How to run it

```bash
rbuilder discover .
rbuilder semantic index                  # or: --embedder vocab|hash
rbuilder -f json semantic query "checkout flow" --limit 10
rbuilder serve --open                    # Search tab (needs index + HTTP API)
```

→ [User Guide §12 — Semantic search](user-guide.md#12-semantic-search)  
→ Design: **[Semantic search design](design/semantic-search-design.md)** · JSON: **[json-api.md § semantic](json-api.md#15-semantic)**

---

## Graph metrics (architecture hotspots)

### Goal

Find **structural hotspots** in the architecture — functions that are central, bridge modules, or form natural communities.

### Description

**Metrics** runs graph algorithms on the indexed call graph:

- **PageRank** — influential nodes (many important callers/callees)  
- **Betweenness** — bridge nodes on many paths  
- **Harmonic centrality** — reachability closeness (used by the migration planner)  
- **Communities** — densely connected clusters (often packages or subsystems)  
- **Blast scores** — per-function impact from the blast engine (shown in the **Functions** tab)

Discover already computes many analytics during indexing; `metrics` exposes them on demand as JSON or text.

### Key benefits

- **Prioritize refactors** where coupling is highest  
- **Onboarding** — “start reading here” for new engineers  
- **Architecture reviews** with quantitative backing

### How to run it

→ [User Guide §11 — Graph metrics](user-guide.md#11-graph-metrics)  
→ Design: **[Graph metrics design](design/graph-metrics-design.md)**

---

## Migration planner (package roadmap)

### Goal

Answer: **“In what order should we migrate or extract packages, given centrality, blast risk, and call dependencies?”** — a concrete roadmap, not just a hotspot list.

### Description

The **migration planner** aggregates per-function metrics into **package-level macro nodes** (Java-style package paths, Rust/C `/src/` module paths), builds a **call graph between packages**, and ranks packages with a tunable score:

`Priority = α·PageRank + β·Harmonic − γ·Blast`

Two orderings are available:

- **Scheduled step** — Kahn topological sort so callees appear before callers (dependency-aware)  
- **Priority rank** — score-only ordering without dependency constraints  

Strategy **presets** (Hybrid Default, Foundational First, Dense Cluster Extraction, Risk Mitigation) adjust α/β/γ. The dashboard **Migration** tab lets you tune weights live and explore a ForceAtlas2 layout (cluster color from **label-propagation communities** — UI field still named `louvain_community_id`; see [community naming](design/graph-metrics-design.md#31-community-detection-naming) — node size from priority). **With `--with-dashboard`**, discover writes `migration_graph.json` and a default `migration_plan.json` under **`.rbuilder/dashboard/`** when analysis metrics are available. Use `--export-migration-hints` for a preset-tuned plan file (default **`.rbuilder/migration_plan.json`**, or `-o`). Dashboard and migration JSON are **opt-in**.

### Key benefits

- **Actionable batches** — migrate by package, not anonymous community ids  
- **Risk-aware ordering** — balance architectural importance against blast impact  
- **Dual views** — strict dependency schedule vs. pure priority for planning debates  
- **Agent-ready JSON** — same plan the dashboard shows, exportable at discover time

### How to run it

```bash
rbuilder discover . --with-cfg --with-security --with-taint --with-dashboard --with-harmonic --export-migration-hints
rbuilder serve --open   # http://127.0.0.1:8080/ → Migration tab + query API
```

→ Engineering detail: **[Migration planner design](design/migration-planner-design.md)** · **[All feature designs](design/README.md)**  

---

## Export and sharing

### Goal

**Take the graph (or a subgraph) out of rBuilder** for other tools — spreadsheets, GraphML viewers, documentation, or custom pipelines.

### Description

**Export** writes files in common formats: JSON (full graph), GraphML, Graphviz DOT, or Mermaid. Use **`--query`** with filter syntax (`name:Symbol`, `type:Function`, `functions`, `all`) — not full GQL `MATCH` (use `gql` for that).

### Key benefits

- **Interop** with existing visualization and graph tools  
- **Snapshots** for compliance or architecture baselines  
- **Custom analytics** in Python/R/Excel on exported JSON

### How to run it

→ [User Guide §13 — Export graph projections](user-guide.md#13-export-graph-projections)

---

## CI policy checks

### Goal

**Block merges** when changed code violates blast-radius or governance rules defined in a policy file.

### Description

**`check`** compares functions touched in the current git working tree (or the whole graph if git is unavailable) against a **policy file**. Violations are listed in JSON; exit code `1` signals failure for CI pipelines.

This pairs with blast-radius semantics: policies can encode scale limits, forbidden cross-domain impact, cascade hazards on high-betweenness nodes, and related rules.

### Key benefits

- **Automated governance** in pull-request pipelines  
- **Consistent enforcement** of architecture rules  
- **Machine-readable violations** for bots and dashboards

### How to run it

→ [User Guide §14 — CI policy check](user-guide.md#14-ci-policy-check)  
→ Policy JSON: **[Policy format](policy-format.md)** · Design: **[CI policy checks design](design/ci-policy-checks-design.md)**

---

## HTTP server (`serve`)

### Goal

Serve the **dashboard** and **GQL HTTP API** in one process, or keep a legacy socket daemon for repeated blast-radius calls.

### Description

**`serve`** (default) binds `http://127.0.0.1:8080/` — dashboard at `/`, queries at `POST /api/query`. Use **`serve --daemon`** for the older Unix-socket path (`.rbuilder/query.sock`) that only accelerates blast-radius auto-connect.

### Key benefits

- **One command** to demo the UI and run GQL from curl or agents  
- **Same JSON shapes** as the CLI  
- **Optional legacy daemon** for blast-heavy scripts without HTTP

### How to run it

→ [User Guide §15 — HTTP server (`serve`)](user-guide.md#15-http-server-serve) · [HTTP API](http-api.md)

---

## Dashboard (visual exploration)

### Goal

**Explore** the graph interactively in a browser — package overview, drill-down, CFG, slice, blast radius, dataflow, taint, and **Migration** (package roadmap) — without memorizing CLI syntax.

### Description

After `discover --with-dashboard`, rBuilder writes a static bundle under **`.rbuilder/dashboard/`** (`index.html`, `manifest.json`, graph payload, metagraph, migration indexes when analysis is available, and per-feature indexes for CFG, slice, blast, taint, etc.). Dashboard export is **off by default**. Serve that folder over HTTP (WASM graph engine requires a real server, not `file://`).

The dashboard complements the CLI: same underlying graph and analysis artifacts. The **Migration** tab mirrors the Rust planner in TypeScript for live preset and weight changes. The **Query Guide** tab lists CLI commands for each view.

| Dashboard tab | Companion design doc |
|---------------|----------------------|
| Graph Visualization | [GQL design](design/gql-design.md) |
| Functions | [Graph metrics design](design/graph-metrics-design.md) |
| CFG / PDG Analysis | [CFG design](design/cfg-design.md) |
| Dataflow | [PDG](design/pdg-design.md) · [Dominance](design/dominance-design.md) |
| Program Slicing | [Program slicing design](design/program-slicing-design.md) |
| Blast Radius | [Blast radius design](design/blast-radius-design.md) |
| Taint Analysis | [Taint analysis design](design/taint-analysis-design.md) |
| Migration | [Migration planner design](design/migration-planner-design.md) |

### Key benefits

- **Visual navigation** for large monorepos (package metagraph, zoom, inspector)  
- **Demos and onboarding** for non-CLI users  
- **Query Guide** — tab-aligned CLI cookbook

### How to run it

```bash
rbuilder discover . --with-dashboard   # writes .rbuilder/dashboard/
rbuilder serve --open                  # recommended: dashboard + /api/query
# or: cd .rbuilder/dashboard && python3 -m http.server 8765
```

→ **[Dashboard user guide](dashboard-user-guide.md)** · **[Feature designs](design/README.md)** · Install: [User Guide §1–3](user-guide.md#1-installation)

---

## Where to go next

| If you want to… | Read |
|-----------------|------|
| Install, PATH, ecommerce-java walkthrough, every command | **[User Guide](user-guide.md)** |
| Use the browser dashboard | **[Dashboard user guide](dashboard-user-guide.md)** |
| Integrate an LLM agent | **[AGENTS.md](../AGENTS.md)** · **[Agent recipes](agent-recipes.md)** |
| Parse `-f json` in scripts or CI | **[JSON API](json-api.md)** |
| HTTP `serve` and `/api/query` | **[HTTP API](http-api.md)** |
| Blast-radius policy files | **[Policy format](policy-format.md)** |
| Exact JSON field tables | **[CLI output schemas](cli-output-schemas.md)** |
| Blast radius (change impact) | **[Blast radius design](design/blast-radius-design.md)** |
| Program slicing / taint | **[Slicing](design/program-slicing-design.md)** · **[Taint](design/taint-analysis-design.md)** |
| CFG / PDG / dominance | **[CFG](design/cfg-design.md)** · **[PDG](design/pdg-design.md)** · **[Dominance](design/dominance-design.md)** |
| Hybrid CPG (mutations / flows) | **[User Guide §10](user-guide.md#10-hybrid-cpg-cpg)** · **[hybrid-cpg-plan](design/hybrid-cpg-plan.md)** |
| GQL and graph exploration | **[GQL design](design/gql-design.md)** |
| Graph metrics | **[Graph metrics design](design/graph-metrics-design.md)** |
| Plan a package-by-package migration roadmap | **[Migration planner design](design/migration-planner-design.md)** · **[Building a migration plan](building-migration-plan.md)** |
| CI policy and governance | **[CI policy design](design/ci-policy-checks-design.md)** · **[Policy format](policy-format.md)** |
| All feature engineering designs (screenshots) | **[design/README.md](design/README.md)** |
| Dashboard engineering / WASM phases | **[Dashboard design](dashboard-design.md)** |
| Blast-radius caches and automated perf gates | **[CLI I/O sanity QE](cli-io-sanity-qe.md)** · **[Graph storage architecture](graph-storage-architecture.md)** |
| All docs by persona | **[Documentation index](README.md)** |
| Papers implemented, inspired, and contribution ideas | **[Further reading](further-reading.md#research-foundations-in-rbuilder)** |

**Suggested first hour**

1. Read this introduction (you are here).  
2. Follow [User Guide §1–4](user-guide.md#1-installation) — install, index [`rbuilder-tests/ecommerce-java`](../rbuilder-tests/ecommerce-java) (`discover`), try CoolStore `/services/*` + `cpg mutations --type ShoppingCart`.  
3. Run one **GQL** query and one **blast-radius** on a function you recognize.  
4. Optionally open the **dashboard** (try the **Migration** tab after `discover --with-cfg --with-security --with-taint --export-migration-hints`) or try `-f json` with [JSON API](json-api.md).
