---
name: rbuilder
description: >-
  Answer structural questions about a codebase using the rBuilder CLI graph
  (architecture, communities, call relationships, blast radius, data-flow
  slices, CPG, semantic search, migration, CI gates). Use when the user asks
  how code is connected, what calls what, impact of changing a symbol,
  where data flows, repo structure/hotspots, or when `.rbuilder/` exists —
  treat natural-language codebase questions as rBuilder queries first.
---

# rBuilder

Answer **structural** questions from a pre-built code knowledge graph instead of reading whole files into context. Prefer `rbuilder -f json …` on **stdout** (parse `schema_version` + payload). Never scrape stderr for JSON. **Never use `2>/dev/null`** — it swallows rBuilder errors (ambiguous symbols, invalid edge types) and causes downstream parse failures on empty output.

Typical user turns are natural language (“Where is the checkout flow?”), not CLI strings. Map intent → command → summarize.

## Agent loop

```text
1. USER PROMPT     → natural language (not a CLI string)
2. TOOL CALL       → rbuilder -f json <command> …  (or discover / semantic index when needed)
3. GRAPH FACTS     → parse schema_version + payload from stdout (or read plan/export files)
4. LLM REASONING   → summarize using “Agent should report” fields
5. ACTION          → edit / plan / check — re-query if the graph may be stale
```

**Prerequisite (once per repo):** `discover` (and `semantic index` when using Search). Deep analysis (`cpg`, `inspect`, slice/taint) needs `discover --with-cfg` (and related flags).

## What you must do when invoked

1. **Help-only** — If the user only wants help / command list / how to use this skill → print the NL table + Usage below and **stop** (no discover, no queries).
2. **Fast path — existing index** — If `.rbuilder/` exists (especially `graph.snapshot.bin`) **and** the request is a structural NL question (not an explicit rebuild) → **do not re-run discover**. Route via the NL table; run matching command(s) with `-f json`.
3. **No index** — Run `rbuilder discover .`. Add flags only when needed (`--with-cfg` for slice/inspect/cpg PDG; `--with-taint` / `slice --taint` for security; migration flags for migration plans; `semantic index` before semantic query).
4. **Natural-language routing** — Map the utterance with the table below. Do not ask the user to rephrase into CLI unless disambiguation is required (`--class` / `--file` on blast-radius).
5. **Summarize** — Report fields under each command’s **Agent should report**. Never dump full JSON unless asked.
6. **Stop conditions** — Pure code-edit/debug with no structural need → do not force rBuilder. On failure, use the Failure playbook.

**Relationship questions** (e.g. “relationship between X and Y”): resolve symbols (GQL / semantic) → bounded `CALLS`/`DEPENDSON` traversal → answer in plain language (hops, shared neighbors, files). If no direct path but asymmetric dependency, fall back to `blast-radius` on each.

## NL → command decision table

| # | User says (patterns) | Prefer |
|---|-------------------------------------|--------|
| 1 | Generate a migration plan / modernize this repo | `discover . --with-cfg --with-security --with-taint --with-dashboard --with-harmonic --export-migration-hints` then **read** `.rbuilder/migration_plan.json` (stdout is discover telemetry, not the plan) |
| 2 | Bottlenecks / central dependencies / hotspots | `-f json metrics --pagerank` → `.pagerank.top` |
| 3 | Inventory of functions / candidates to delete or shrink | `-f json gql --macro-name all_functions unused` — **`unused` is a required QUERY placeholder**, not “find dead code”; follow up with blast-radius / CALL queries |
| 4 | What communities / packages does the graph see? | `-f json gql --macro-name all_communities unused` (lists communities, **not** orphans) or prefer `communities list` |
| 5 | Export GraphSON archive before refactor | `cpg export --format graphson --output cpg.json [--path-contains src/]` (writes a **file**; needs `--with-cfg` for rich L_proc) |
| 6 | Where is the checkout flow? / NL find | `semantic index` (if needed) then `-f json semantic query "checkout flow" --limit 10` |
| 7 | Which subsystem owns checkout? | `-f json semantic query "checkout" --scope community --limit 10` |
| 8 | Find all *Service* / naming patterns | `-f json gql "MATCH (n:Function) WHERE n.name LIKE '*Service' RETURN n LIMIT 20"` (suffix only — `*middle*` silently returns 0; for contains, use `semantic query`) |
| 9 | List functions in community N | `-f json gql "MATCH (f:Function) WHERE f.community_id = '12' RETURN f LIMIT 20"` |
| 10 | Impact if I change `updateQuantity` | `-f json blast-radius updateQuantity --depth 2` (add `--class` / `--file` if ambiguous) |
| 11 | Call stack / neighborhood up to 3 hops | `-f json gql "MATCH (a:Function)-[:CALLS*1..3]->(b:Function) WHERE a.name = 'updateQuantity' RETURN a,b LIMIT 50"` |
| 12 | AST skeleton / structure of a function | `discover . --with-ast-skeleton` then `-f json cpg ast updateQuantity` (coarse `kind`/lines/label — **not** typed params/return) |
| 13 | CFG archive ready + slice `quantity` in `updateQuantity` | `-f json cpg status` then `-f json cpg slice FILE --line N --variable quantity --function updateQuantity [--view pdg]` (**no** `--symbol`) |
| 14 | Where is `ShoppingCart` mutated? | `-f json cpg mutations --type ShoppingCart --exclude-ctors` |
| 15 | Trace variable `quantity` / data flow | `-f json cpg flows FILE --line N --variable quantity --function updateQuantity --direction forward` |
| 16 | Loop-carried / parallelization hazards | `discover . --with-cfg --with-dfg-loops` then `-f json inspect <Symbol> pdg --edge-layer data` (look for `loop_carried`) |
| 17 | Validate against policy before commit | `-f json check --policy-file policy.json` (blast-radius policy schema — see `docs/policy-format.md`) |
| — | relationship between X and Y | `gql` CALLS/DEPENDSON path (see encyclopedia) |
| — | who calls X / what X calls | incoming vs outgoing GQL (or blast-radius for impact) |
| — | open dashboard / many queries | `serve --open` |

**Accuracy notes:** Prefer `-f json`. Macro `unused` ≠ unused-code analysis. Migration plan is the **file**. `cpg slice` needs file + line + variable. Policy rules are not free-form named ids.


## Usage

```bash
export REPO=/path/to/repo   # directory that contains or will contain .rbuilder/
rbuilder -r "$REPO" discover .
rbuilder -r "$REPO" -f json <command> …
```

Globals: `-f json` (agents), `-r` / `--repo`, `-o` file. Workflow: **discover once → query many**.

## Artifacts

| Path | Content |
|------|---------|
| `.rbuilder/graph.snapshot.bin` | Graph snapshot |
| `.rbuilder/dashboard/manifest.json` | Counts, feature flags |
| `.rbuilder/dashboard/migration_plan.json` | Migration export |
| `.rbuilder/dashboard/graph_payload.bin` | Dashboard WASM graph |
| `.rbuilder/semantic_index.bin` | Semantic index (`semantic index`) |
| `.rbuilder/migration_plan.json` | Migration plan (with `--export-migration-hints`) |
| `.rbuilder/analysis/` | CFG/PDG archives (with `--with-cfg`) |

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Policy violation (`check`, `blast-radius --policy-file`) or command error |

## Failure playbook

| Symptom | Fix |
|---------|-----|
| No `.rbuilder/` / graph missing | `rbuilder discover .` |
| slice / inspect / cpg PDG fails | Re-discover with `--with-cfg` |
| semantic query fails | `rbuilder semantic index` (offline: `--embedder vocab` or `hash`) |
| Ambiguous symbol | `--class` / `--file` on blast-radius; disambiguate via GQL |
| `check` / policy exit 1 | Report violations (JSON still on stdout when applicable) |
| `inspect` / `slice --function` confusion | `inspect` = symbol only; `slice --function` = **method** name |
| `export --query` with MATCH | Use filter syntax (`name:Foo`, `type:Function`, `all`) |
| blast-radius returns 0 / CALLS returns 0 edges for a method with known callers | Interface / dynamic dispatch (receiver methods, virtual calls, trait impls) is invisible to the static call graph. Fall back to `grep` for call sites in source |
| GQL LIKE returns 0 results | Concepts often live in package/directory paths or type names, not bare function names. Try `communities list` and grep labels, or `semantic query`, before concluding nothing exists |

---

## Command encyclopedia

Samples below are truncated. Field names match live CLI / `docs/cli-output-schemas.md`. Fixture: `rbuilder-tests/ecommerce-java` unless noted **illustrative** (schema-faithful shape).

### `discover`

**Command:** `rbuilder [-f json] discover [PATH] [--with-cfg] [--with-security] [--with-taint] [--with-dashboard] [--with-harmonic] [--export-migration-hints] [--with-ast-skeleton] [--with-dfg-loops] …`

**Purpose:** Index the repo once (or after large changes). Build the graph agents query.

**Prerequisites:** None (this creates `.rbuilder/`).

**Sample** (`-f json`, ecommerce-java):

```json
{
  "schema_version": 2,
  "command": "discover",
  "metrics": {
    "files_discovered": 66,
    "files_indexed": 66,
    "files_skipped": 0,
    "nodes_generated": 843,
    "edges_generated": 1793,
    "duration_ms": 306
  }
}
```

**Pitfalls:** Do not re-run on every question if `.rbuilder/` exists. `--with-cfg` needed for slice/inspect/cpg PDG. `--with-taint` is discover-time taint (on-demand: `slice --taint`). Semantic search needs a separate `semantic index`.

**Agent should report:** files indexed, nodes/edges, duration; note which feature flags were used.

### `gql`

**Command:** `rbuilder -f json gql '<MATCH…>'` or `rbuilder -f json gql --macro-name <NAME> unused`

**Purpose:** Inventory, callers/callees, communities, path/relationship queries.

**Prerequisites:** `discover` done. Virtual `:Community` needs analysis overlay from discover.

**Sample** (macro `all_functions`):

```json
{
  "schema_version": 1,
  "count": 260,
  "rows": [
    [{ "binding": "f", "node": "addItem", "type": "Function",
       "file": "…/controller/CartController.java" }]
  ],
  "explain": false
}
```

**Useful patterns:**

```bash
# Incoming callers of X
rbuilder -f json gql "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE b.name = 'checkout' RETURN a,b LIMIT 20"
# Outgoing callees of X
rbuilder -f json gql "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE a.name = 'checkout' RETURN a,b LIMIT 20"
# Name search (prefix or suffix only — *middle* silently returns 0)
rbuilder -f json gql "MATCH (n:Function) WHERE n.name LIKE '*Service' RETURN n LIMIT 20"
# Communities macro
rbuilder -f json gql --macro-name all_communities unused
```

**Pitfalls:** `--macro-name` still needs a positional query arg — pass `unused`. `--explain` plan is text-mode only. rBuilder GQL is a **subset of Cypher** — no `COUNT`, `ORDER BY`, `GROUP BY`, or aggregation functions. CALLS edges are static — interface / dynamic dispatch (receiver methods, virtual calls, trait impls) may not appear; if a `CALLS*1..N` query returns 0 edges for a method you know is called, fall back to `grep` for call sites. If LIKE on function names returns 0 for a concept (e.g. "ingress", "gateway"), it likely lives in package/directory names, type names, or community labels — try `communities list`, `semantic query`, or broaden the LIKE to non-Function node types before concluding nothing exists.

**Agent should report:** matching symbols, files, hop relationships — not raw row dumps.

### `blast-radius`

**Command:** `rbuilder -f json blast-radius '<Symbol>' [--depth N] [--class C] [--file P] [--with-slices] [--policy-file PATH]`

**Purpose:** Upstream change impact — who breaks if this symbol changes.

**Prerequisites:** `discover` done.

**Sample** (schema v2 shape; ecommerce names — field set matches live CLI):

```bash
rbuilder -f json blast-radius 'checkout' --class OrderService --depth 3
```

```json
{
  "schema_version": 2,
  "target": {
    "id": "424d403b-1b2c-4a3d-8e9f-0c1b2a3f4e5d",
    "symbol": "checkout",
    "class_context": "OrderService",
    "file_path": "…/service/OrderService.java",
    "language": "java",
    "signature": "public OrderDto checkout() {",
    "canonical_fqn": "OrderService::checkout"
  },
  "metrics": {
    "score": 25.05,
    "direct_callers_count": 1,
    "impact_zone_size": 3,
    "caller_depth_limit": 3
  },
  "topology": {
    "scc_component_id": null,
    "direct_callers": [
      {
        "id": "8b2c4a3d-0c1b-4e5d-8e9f-424d403b1b2c",
        "fqn": "OrderController.checkout",
        "file_path": "…/OrderController.java"
      }
    ],
    "impact_zone": [
      {
        "id": "…",
        "fqn": "…",
        "file_path": "…"
      }
    ]
  },
  "gatekeeping": { "policy_status": "SKIPPED", "violations": [], "handoffs": [] }
}
```

**Pitfalls:** Ambiguous names need `--class` / `--file`. `--with-slices` is slow. Exit `1` when policy `VIOLATED` (JSON still emitted first). **Interface / dynamic dispatch:** blast-radius and CALLS edges track static call sites only — receiver methods, virtual calls, and trait/interface impls may return score=0 / 0 callers even when widely used. If blast-radius returns 0 for a method that clearly has callers, fall back to `grep` for call sites in source.

**Agent should report:** score, direct callers (`fqn` / `file_path`), impact_zone_size, policy status — not full topology arrays. Ignore or pass through extra v2 fields (`id`, `language`, `signature`, `scc_component_id`) as needed.

### `slice`

**Command:** `rbuilder -f json slice <FILE> --line N --variable V [--function METHOD] [--direction backward|forward] [--taint] [--view text|cfg|pdg]`

**Purpose:** Line-level data dependence (what affects V / where V flows). `--taint` for source→sink security.

**Prerequisites:** Prefer `discover --with-cfg`. `--function` is the **method/function name**, not the class.

**Sample** (ecommerce-java `CartService.addItem`):

```bash
rbuilder -f json slice src/main/java/com/example/ecommerce/service/CartService.java \
  --line 38 --variable cart --function addItem --direction backward
```

```json
{
  "schema_version": 1,
  "file": "src/main/java/com/example/ecommerce/service/CartService.java",
  "direction": "backward",
  "criterion": { "line": 38, "variable": "cart" },
  "lines": [38],
  "reduction_percent": 92.86,
  "nodes": [
    {
      "id": "node_0",
      "kind": "Expression",
      "label": "Cart cart = getUserCart();",
      "line": 38
    }
  ],
  "edges": []
}
```

**Pitfalls:** Wrong `--function` (class vs method) is a common failure. Needs CFG archive.

**Agent should report:** criterion, direction, `nodes[].label` / lines, reduction — not the full edge list unless asked.

### `inspect`

**Command:** `rbuilder -f json inspect <SYMBOL> cfg|pdg|dom`

**Purpose:** Raw CFG / PDG / dominator view for one function.

**Prerequisites:** `discover --with-cfg`. Symbol only — **no** `--class` (disambiguate via blast-radius / GQL first).

**Sample:**

```json
{
  "schema_version": 1,
  "symbol": "addItem",
  "layer": "cfg",
  "pruned": false,
  "nodes": [
    {
      "id": "block_0",
      "block_index": 0,
      "start_line": 0,
      "end_line": 0,
      "statements": []
    },
    {
      "id": "block_1",
      "block_index": 1,
      "start_line": 24,
      "end_line": 24,
      "statements": [
        { "kind": "Return", "line": 24, "text": "return cartService.addItem(…);" }
      ]
    }
  ],
  "edges": [
    { "kind": "return", "source": "block_1", "target": "block_0" }
  ]
}
```

There are **no** `nodes_count` / `edges_count` fields — use `len(nodes)` / `len(edges)`.

**Pitfalls:** Ambiguous symbols fail; resolve FQN/name carefully. CFG may be **partial** on complex methods (covering only the first branch/entry block) — supplement with source reading if the block count seems low for the method's complexity.

**Agent should report:** layer, `len(nodes)` / `len(edges)`, notable `statements[].text` — not every node. Note if the CFG appears incomplete.

### `metrics`

**Command:** `rbuilder -f json metrics [--pagerank] [--betweenness] [--communities] [--iterations N]`

**Purpose:** Hotspots (PageRank), bridges (betweenness), community stats.

**Prerequisites:** `discover` done. Default (no flags) computes all sections.

**Sample** (`--pagerank`; `top[]` entries are node UUIDs — **not names**):

```json
{
  "schema_version": 1,
  "pagerank": {
    "top": [{ "node": "<uuid>", "pagerank": 0.0117 }],
    "converged": true,
    "iterations": 20,
    "max_delta": 0.0
  }
}
```

**Resolving UUIDs to names:** PageRank covers all node types (Functions, Modules, Classes). To get the actual name/file for a UUID:

```bash
# For Function nodes (cheap, O(1)):
rbuilder -f json cpg function '<uuid>'
# For any node type (heavier but always works):
rbuilder -f json blast-radius '<uuid>'
```

Loop over `top[]` UUIDs and resolve each. GQL `WHERE n.id = '<uuid>'` does **not** work (node id is not a queryable property).

**Agent should report:** top hotspot symbols (resolve UUIDs first), modularity/community count when requested.

### `semantic`

**Command:**

```bash
rbuilder semantic index [--embedder code-daemon|vocab|hash]
rbuilder -f json semantic query "…" [--limit N] [--scope community]
```

**Purpose:** Natural-language / keyword find of functions (and community-scoped search).

**Prerequisites:** `discover`, then **`semantic index`** (separate artifact). Offline: `--embedder vocab` or `hash`. Default code-daemon needs ONNX weights.

**Sample** (`--embedder vocab`, query `checkout cart`):

```json
{
  "schema_version": 3,
  "query": "checkout cart",
  "model_id": "vocab-accumulate-v1",
  "dimensions": 256,
  "index_schema_version": 1,
  "hits": [
    {
      "name": "getCart",
      "qualified_name": "CartController.getCart",
      "node_id": "94823a58-9efd-4de4-95fb-aa082c2012c3",
      "score": 0.50,
      "fused_score": 0.50,
      "distance": 40,
      "ranking": "fusion",
      "file_path": "…/CartController.java"
    }
  ]
}
```

**Pitfalls:** Query without index fails. Restart `serve` after rebuilding index for dashboard search. **Large repos (100K+ nodes):** `--scope community` may return only singleton communities because label-propagation produces very granular clusters. For subsystem ownership on large repos, prefer `communities list` + grep labels over `--scope community`.

**Agent should report:** top hit names, files, scores (`score` / `fused_score`); keep `node_id` for follow-up GQL — not every hit.

### `communities`

**Command:** `rbuilder -f json communities list` (and refresh subcommands per `--help`)

**Purpose:** Named community overlay (subsystems).

**Prerequisites:** `discover` (community detection during analysis).

**Sample:**

```json
{
  "schema_version": 1,
  "modularity": 0.45,
  "written": false,
  "communities": [
    { "id": 462, "label": "ecommerce.service::findByEmail", "member_count": 19 }
  ]
}
```

**Agent should report:** top labels + sizes; use GQL `community_id` for members.

### `cpg`

**Command:** `rbuilder -f json cpg <subcommand> …`

**Purpose:** Hybrid CPG façade (repo topology + CFG/PDG archive).

**Prerequisites:** `discover`; **`--with-cfg`** for PDG/slice/mutations/flows; `--with-ast-skeleton` for `ast`.

#### `cpg status`

```bash
rbuilder -f json cpg status
```

**Purpose:** Is the L_proc / CFG–PDG archive ready?

**Agent should report:** ready/not ready; whether to re-run `discover --with-cfg`.

#### `cpg function` / `cpg calls`

```bash
rbuilder -f json cpg function '<Symbol>'
rbuilder -f json cpg calls '<Symbol>'
```

**Purpose:** Resolve a function in L_repo and whether L_proc exists; CALL neighborhood.

**Agent should report:** resolved identity + direct call neighbors.

#### `cpg pdg` / `cpg slice` / `cpg flows`

```bash
rbuilder -f json cpg pdg '<Symbol>'
rbuilder -f json cpg slice …   # wraps slice; see slice flags
rbuilder -f json cpg flows FILE --line N --variable V --function F \
  [--direction forward|backward] [--with-alias]
```

**Purpose:** Dependence / data-flow overlays (prefer these when already in a CPG workflow).

**Pitfalls:** Missing archive → re-discover `--with-cfg`. `--with-alias` expands may-alias names. `--line` must point to a line **inside a function body** — struct definitions, import blocks, or other non-function lines will fail or return empty results.

**Agent should report:** key dependent statements / flow direction — not full graphs.

#### `cpg mutations`

```bash
rbuilder -f json cpg mutations --type ShoppingCart [--exclude-ctors]
```

**Purpose:** Field mutations on a type (cart / DTO safety).

**Prerequisites:** `discover --with-cfg`.

**Agent should report:** which fields are written, by which functions.

#### `cpg ast`

```bash
rbuilder -f json cpg ast '<Symbol>'
```

**Purpose:** Coarse AST skeleton for a function.

**Prerequisites:** `discover --with-ast-skeleton`.

**Agent should report:** skeleton summary / notable nodes.

#### `cpg export`

```bash
rbuilder cpg export --format graphson --output cpg.json [--path-contains src/]
```

**Purpose:** Export hybrid CPG view (GraphML / GraphSON).

**Agent should report:** output path + format.

**General cpg pitfalls:** Archive IO errors mean re-run `discover --with-cfg` (and ensure write permissions under `.rbuilder/analysis/`).

### `check`

**Command:** `rbuilder -f json check --policy-file policy.json`

**Purpose:** CI gate — fail when blast-radius policy rules are violated.

**Prerequisites:** `discover`; valid policy file (`docs/policy-format.md`).

**Sample:**

```json
{
  "schema_version": 1,
  "passed": true,
  "policy": "rbuilder-tests/rbuilder-policy.json",
  "violations": []
}
```

**Pitfalls:** Exit code `1` on failure — still parse JSON for violations.

**Agent should report:** passed/failed + violation summaries.

### `export`

**Command:** `rbuilder export --export-format mermaid|graphviz|… --export-output OUT [--query FILTER]`

**Purpose:** Export graph / neighborhood diagrams.

**Prerequisites:** `discover` done.

**Pitfalls (critical):** `--query` uses **filter** syntax — `name:Foo`, `type:Function`, `all` — **not** GQL `MATCH … RETURN`. Agents must not pass MATCH strings to `--query`.

**Agent should report:** output path + format; confirm filter used.

### `serve`

**Command:** `rbuilder serve [--open] [--host] [--port] [--query-only|--dashboard-only]`

**Purpose:** HTTP dashboard + `POST /api/query` (and semantic routes). Preferred for many interactive queries.

**Prerequisites:** `discover` (dashboard bundle with `--with-dashboard` for full UI).

**Pitfalls:** `serve --daemon` is the **legacy** Unix-socket blast-radius daemon — prefer HTTP `serve`. Only use `--daemon` if the user explicitly asks for the old socket.

**Agent should report:** URL/port; how to POST a sample query.

---

### Worked NL scenarios (NL → tool → reason)

Run the commands as shown; parse encyclopedia sample shapes. Summarize — don’t dump payloads.

#### Flow 1 — Modernization & audit

**1. Migration plan** — *“Generate a complete migration plan…”*

```bash
rbuilder discover . --with-cfg --with-security --with-taint \
  --with-dashboard --with-harmonic --export-migration-hints
# read .rbuilder/migration_plan.json (and/or dashboard Migration tab via serve --open)
```

Discover stdout (`-f json`) is **telemetry** — not the plan body. Report path + top `packages[]` by priority/step.

**2. Hotspots** — *“Which core functions are bottlenecks / central dependencies?”*

```bash
rbuilder -f json metrics --pagerank
```

Report `.pagerank.top` nodes + why they are risky to change.

**3. Function inventory** — *“Give me an inventory of functions … candidates to delete or shrink.”*

```bash
rbuilder -f json gql --macro-name all_functions unused
```

`all_functions` → full inventory (`count` + `rows`). `unused` is a **placeholder**. Cross-check with blast-radius / CALL queries before deletes.

**4. Named communities** — *“What architectural communities / packages does the graph see?”*

```bash
rbuilder -f json gql --macro-name all_communities unused
# prefer for labels + modularity: rbuilder -f json communities list
```

Lists communities — **not** “orphaned modules.” Inspect members and call edges before proposing a prune.

**5. CPG export** — *“Export … GraphSON … archive the baseline…”*

```bash
rbuilder cpg export --format graphson --output cpg.json --path-contains src/
```

Writes a **file**; success is typically a text summary. Needs prior `discover --with-cfg` for a useful L_proc-rich export.

#### Flow 2 — Intent discovery & subsystem mapping

**6. NL function search** — *“Where is the code that handles our checkout flow?”*

```bash
rbuilder semantic index                    # opt-in; offline: --embedder vocab|hash
rbuilder -f json semantic query "checkout flow" --limit 10
```

Report top `hits[]` (`name`, `score`, `file_path`).

**7. Community semantic** — *“Which architectural subsystem owns checkout?”*

```bash
rbuilder -f json semantic query "checkout" --scope community --limit 10
```

Hits are pooled **community** results (same `hits[]` contract).

**8. Pattern search** — *“Find all Service classes … naming consistency.”*

```bash
rbuilder -f json gql "MATCH (n:Function) WHERE n.name LIKE '*Service' RETURN n LIMIT 20"
```

Suffix-only — `*middle*` silently returns 0. For contains-style search, use `semantic query "Service"` instead. Graph names are often bare method names; see LIKE limitations in GQL quick reference.

**9. Community members** — *“List all the functions inside Community 12…”*

```bash
rbuilder -f json gql "MATCH (f:Function) WHERE f.community_id = '12' RETURN f LIMIT 20"
```

#### Flow 3 — Pre-refactor safety (`updateQuantity`)

**10. Blast radius** — *“What's the impact if I change the signature of `updateQuantity`?”*

```bash
rbuilder -f json blast-radius updateQuantity --depth 2
```

Report `metrics.score`, `topology.direct_callers`, impact size. Add `--class` / `--file` if ambiguous.

**11. Call neighborhood** — *“Show me the call stack surrounding `updateQuantity` up to 3 hops.”*

```bash
rbuilder -f json gql "MATCH (a:Function)-[:CALLS*1..3]->(b:Function)
  WHERE a.name = 'updateQuantity' RETURN a,b LIMIT 50"
```

**12. AST skeleton** — *“Inspect the AST skeleton of `updateQuantity` to check its structure.”*

```bash
rbuilder discover . --with-ast-skeleton
rbuilder -f json cpg ast updateQuantity
```

Coarse skeleton (`kind`, lines, `label`) — **not** a typed signature API (`params` / `return_type` are not emitted).

**13. Status + line slice** — *“Confirm the CFG archive is ready, then slice how `quantity` is used in `updateQuantity`.”*

```bash
rbuilder -f json cpg status
rbuilder -f json cpg slice src/cart/CartService.ts \
  --line 50 --variable quantity --function updateQuantity --view pdg
```

**`cpg slice` has no `--symbol`.** For whole-function CFG/PDG, use `inspect <Symbol> cfg|pdg` or `cpg pdg <Symbol>`.

**14. Field mutations** — *“Check where `ShoppingCart` object fields are mutated…”*

```bash
rbuilder -f json cpg mutations --type ShoppingCart --exclude-ctors
```

**15. Data flows** — *“Trace how the `quantity` variable flows … into database queries.”*

```bash
rbuilder -f json cpg flows src/cart/CartService.ts \
  --line 50 --variable quantity --function updateQuantity --direction forward
```

**16. Loop-carried DFG** — *“Check … loop-carried dependencies that prevent parallelization.”*

```bash
rbuilder discover . --with-cfg --with-dfg-loops
rbuilder -f json inspect BatchProcessor.process pdg --edge-layer data
```

`--with-dfg-loops` **tags** edges during discover — it does not print a dedicated loop-hazard array. Look for `loop_carried` on PDG data deps.

#### Flow 4 — CI gate

**17. Policy check** — *“Validate … against project policies before committing.”*

```bash
rbuilder -f json check --policy-file policy.json
```

Blast-radius policy schema (`max_impact_nodes`, `forbidden_crossings`, …) — see [docs/policy-format.md](../../docs/policy-format.md). Named rules like `no-controller-direct-db-access` are **not** built-in ids. Report `passed` + `violations`.

#### Extra NL

**Relationship between A and B** — resolve symbols → bounded CALLS/DEPENDSON → prose hops/files; fall back to blast-radius if asymmetric.

**Concept search with 0 LIKE hits** — try `communities list` / `semantic query` before concluding nothing exists.

---

### Discover feature-flag cheat sheet

| Flag | Enables |
|------|---------|
| `--with-cfg` | CFG/PDG/dominance archive (slice, inspect, cpg PDG) |
| `--with-taint` | Discover-time taint (implies CFG as needed) |
| `--with-security` | Secret scanning |
| `--with-dashboard` | `.rbuilder/dashboard/` bundle |
| `--with-harmonic` | Harmonic centrality (migration ranking; expensive) |
| `--export-migration-hints` | Write `migration_plan.json` |
| `--with-ast-skeleton` | AST skeleton for `cpg ast` |
| `--with-dfg-loops` | Tag loop-carried data deps on PDG |

Migration-oriented discover (heavy):

```bash
rbuilder discover . --with-cfg --with-security --with-taint \
  --with-dashboard --with-harmonic --export-migration-hints
# then read .rbuilder/migration_plan.json (or dashboard copy)
```

**Agent should report:** path to plan + top scheduled packages — not the entire JSON.

---

### Macros & GQL quick reference

| Macro / pattern | Intent |
|-----------------|--------|
| `--macro-name all_functions unused` | Inventory functions |
| `--macro-name all_communities unused` | List communities |
| `CALLS` / `CALLS*1..3` | Caller/callee chains |
| `n.name LIKE 'Handle*'` | Prefix search (starts-with) |
| `n.name LIKE '*Handler'` | Suffix search (ends-with) |
| `f.community_id = '12'` | Community members |

Always pass a positional query string with `--macro-name` (use `unused` if unused).

**GQL is a Cypher subset:** `MATCH`, `WHERE`, `RETURN`, `LIMIT` only. No `COUNT`, `ORDER BY`, `GROUP BY`, `COLLECT`, or other aggregation. No `WHERE n.id = '<uuid>'` (node UUID is not a queryable property — use `cpg function` or `blast-radius` to resolve UUIDs).

**Valid edge types:** `CALLS`, `CONTAINS`, `USES`, `IMPLEMENTS`, `EXTENDS`, `REFERENCES`, `INSTANTIATES`, `MODIFIES`, `USESCONFIG`, `DEFINEDIN`, `DEPENDSON`. There are no `DEPENDS` or `IMPORTS` edge types.

**LIKE limitations:** Only single-sided wildcards work — `prefix*` (starts-with) or `*suffix` (ends-with). **`*middle*` (contains) silently returns 0.** `WHERE n.file LIKE ...` also returns 0 (file path is not filterable in WHERE clauses). For substring/concept search, use `communities list`, `semantic query`, or `blast-radius` with `--file` instead.

---

### Policy + HTTP session notes

**CI policy** — keep a `policy.json` in-repo; agents should run `check` before claiming a change is merge-safe when the user asks for gates.

**HTTP session** — for many queries in one task:

```bash
rbuilder -r "$REPO" serve --open
# POST http://127.0.0.1:8080/api/query
# {"query":"MATCH (n:Function) RETURN n LIMIT 5"}
```

See [docs/http-api.md](../../docs/http-api.md). Prefer this over `serve --daemon`.

---

## See also

- Full field tables: [docs/cli-output-schemas.md](../../docs/cli-output-schemas.md)
- TypeScript-oriented shapes: [docs/json-api.md](../../docs/json-api.md)
- Copy-paste recipes: [docs/agent-recipes.md](../../docs/agent-recipes.md)
- User guide: [docs/user-guide.md](../../docs/user-guide.md)
- HTTP API: [docs/http-api.md](../../docs/http-api.md)
- Policy format: [docs/policy-format.md](../../docs/policy-format.md)

**Other repos / OpenCode:** copy or symlink this `skills/rbuilder` directory into that project’s `.claude/skills/` or `.cursor/skills/` (or the runtime’s skills path).
