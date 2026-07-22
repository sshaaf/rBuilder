# rBuilder User Guide

End-to-end guide for installing rBuilder, indexing an in-tree example, and querying a codebase from the **command line**. Every command below includes sample output captured against **`rbuilder-tests/ecommerce-java`** (Spring Boot e-commerce fixture).

**New to code graphs?** Read **[Introduction](Introduction.md)** first — concepts, goals, and benefits for each feature, with links back here for commands.

For JSON field reference see [cli-output-schemas.md](cli-output-schemas.md) and [json-api.md](json-api.md).

---

## Table of contents

1. [Installation](#1-installation)
2. [Add rBuilder to your PATH](#2-add-rbuilder-to-your-path)
3. [Example project: ecommerce-java](#3-example-project-ecommerce-java)
4. [Index with `discover`](#4-index-with-discover)
5. [Global CLI flags](#5-global-cli-flags)
6. [Query the graph with GQL](#6-query-the-graph-with-gql)
7. [Blast radius (change impact)](#7-blast-radius-change-impact)
8. [Program slicing and taint](#8-program-slicing-and-taint)
9. [Inspect CFG / PDG / dominance](#9-inspect-cfg--pdg--dominance)
10. [Hybrid CPG (`cpg`)](#10-hybrid-cpg-cpg)
11. [Graph metrics](#11-graph-metrics)
12. [Semantic search](#12-semantic-search)
13. [Export graph projections](#13-export-graph-projections)
14. [CI policy check](#14-ci-policy-check)
15. [HTTP server (`serve`)](#15-http-server-serve)
16. [Recommended workflow](#16-recommended-workflow)
17. [Command reference](#17-command-reference)
18. [Troubleshooting](#18-troubleshooting)

---

## 1. Installation

### Option A — GitHub release (recommended)

Pre-built binaries are published on the project **Releases** page:

**https://github.com/sshaaf/rBuilder/releases**

1. Open the latest release.
2. Download the archive for your platform:

   | Platform | Typical asset name |
   |----------|-------------------|
   | macOS (Apple Silicon) | `rbuilder-*-aarch64-apple-darwin.tar.gz` |
   | macOS (Intel) | `rbuilder-*-x86_64-apple-darwin.tar.gz` |
   | Linux (x86_64) | `rbuilder-*-x86_64-unknown-linux-gnu.tar.gz` |
   | Windows | `rbuilder-*-x86_64-pc-windows-msvc.zip` |

3. Extract the archive. You should get a single `rbuilder` executable (plus `rbuilder.exe` on Windows).

```bash
# macOS / Linux example
tar -xzf rbuilder-*-aarch64-apple-darwin.tar.gz
./rbuilder --version
```

```powershell
# Windows example (PowerShell)
Expand-Archive rbuilder-*-x86_64-pc-windows-msvc.zip -DestinationPath .
.\rbuilder.exe --version
```

If no release is published yet for your platform, use [Option B](#option-b--build-from-source).

### Option B — Build from source

Requires **Rust 1.70+** ([rustup.rs](https://rustup.rs/)).

```bash
git clone https://github.com/sshaaf/rBuilder.git
cd rBuilder
cargo build --release
./target/release/rbuilder --version
```

All seven Tier 1 languages (Rust, Python, JavaScript, TypeScript, Go, Java, C#) are always included in the binary.

---

## 2. Add rBuilder to your PATH

Pick one approach for your shell.

### macOS / Linux — user-local install

```bash
mkdir -p ~/.local/bin
cp /path/to/rbuilder ~/.local/bin/
chmod +x ~/.local/bin/rbuilder
```

Add to `~/.zshrc` or `~/.bashrc`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Reload and verify:

```bash
source ~/.zshrc   # or ~/.bashrc
rbuilder --version
```

### macOS / Linux — system-wide (optional)

```bash
sudo cp /path/to/rbuilder /usr/local/bin/
rbuilder --version
```

### Windows

1. Copy `rbuilder.exe` to a folder such as `C:\Tools\rbuilder\`.
2. Open **Settings → System → About → Advanced system settings → Environment Variables**.
3. Under **User variables**, edit `Path` and add `C:\Tools\rbuilder`.
4. Open a new terminal:

```powershell
rbuilder --version
```

### Per-project usage (no PATH change)

Pass the full path or use a repo-local alias:

```bash
alias rbuilder='/path/to/rbuilder'
```

---

## 3. Example project: ecommerce-java

This guide uses the in-tree Spring Boot fixture shipped with rBuilder:

**[`rbuilder-tests/ecommerce-java`](../rbuilder-tests/ecommerce-java)**

It implements the same e-commerce domain as the other `ecommerce-*` fixtures (cart, orders, products, auth), plus a **CoolStore-compatible dual API** under `/services/*` (additive next to `/api/*`). No separate clone is required when you have the rBuilder repo.

```bash
# From the rBuilder repository root
export REPO="$PWD/rbuilder-tests/ecommerce-java"
cd "$REPO"
```

Layout (simplified):

```
ecommerce-java/
├── pom.xml
└── src/main/java/com/example/ecommerce/
    ├── controller/     # /api/* — CartController, OrderController, ProductController, …
    ├── service/        # CartService, OrderService, ProductService, …
    ├── entity/         # Cart, Order, Product, User, …
    ├── repository/     # Spring Data JPA repos
    ├── security/       # JWT filter / token provider
    └── coolstore/      # /services/* — CoolStore cart pricing + orders (in-memory)
        ├── rest/       # ProductEndpoint, CartEndpoint, OrderEndpoint
        ├── service/    # ShoppingCartService, PromoService, ShippingService, …
        └── model/      # ShoppingCart, ShoppingCartItem, CatalogProduct, …
```

**Dual REST surface** (same contract on every `ecommerce-*` language):

| Surface | Role |
|---------|------|
| `/api/*` | JWT e-commerce API (auth, categories, cart ownership, reviews, …) |
| `/services/*` | CoolStore-style products / cart / checkout / orders (`cartId` session carts) |

`ShoppingCartService.priceShoppingCart` mutates cart totals (promo + shipping) — the Layer F target for `cpg mutations --type ShoppingCart`. Full route table: [`rbuilder-tests/README.md`](../rbuilder-tests/README.md).

Sibling fixtures (`ecommerce-python`, `ecommerce-rust`, `ecommerce-c`, …) share both REST shapes.

All commands below assume `REPO` points at `ecommerce-java`, or that you run from inside that directory and use `.` instead of `"$REPO"`.

**Sample outputs** in this guide were captured on a laptop with a release build; absolute paths are shortened to `…/ecommerce-java/…` for readability. Counts may differ slightly across versions.

---

## 4. Index with `discover`

`discover` scans source files, builds the knowledge graph, runs analytics (complexity, communities, centrality, blast-radius scoring), and writes artifacts under `.rbuilder/`.

### Fast index (default)

```bash
cd "$REPO"
rbuilder discover . -l java -e target
```

Example output:

```text
==> Analyzing: …/ecommerce-java/.
[✓] Indexed 51 files -> 518 nodes, 1122 edges (0.0s)
[✓] Detected 443 communities (modularity: 0.47)
[✓] Analyzed 187 functions (avg complexity: 1.0, 0 high, 0 medium)
[*] Top hotspot: findAll (PageRank: 0.0177)
[!] Found 48 circular dependencies
[✓] Analysis complete
[✓] Saved to .rbuilder/ (0.1 MB total)
[✓] Completed in 0.0s (peak memory: 21 MB)

[i] Next steps:
   rbuilder gql "MATCH (n:Function) RETURN n"  # Query the graph
   rbuilder slice <file> --line <N> --variable <VAR>
   rbuilder serve --open   # Dashboard + query API at http://127.0.0.1:8080
```

Typical runtime on this fixture: **well under a second**.

**CI / automation** — structured metrics on stdout:

```bash
rbuilder -f json discover . -l java -e target | jq .
```

Example:

```json
{
  "command": "discover",
  "metrics": {
    "duration_ms": 32,
    "edges_generated": 1122,
    "files_discovered": 51,
    "files_indexed": 51,
    "files_skipped": 0,
    "nodes_generated": 518
  },
  "schema_version": 2
}
```

### Language and path filters

```bash
# Java only, skip Maven output
rbuilder discover . -l java -e target

# Multiple languages (polyglot monorepo)
rbuilder discover . -l java,typescript -e target,node_modules,dist
```

### Default pipeline (always on)

Bare `discover` (no `--with-*`) always runs: index/extract → topology → community → complexity → PageRank/betweenness → dependency cycles → blast engine → persist analysis + snapshot.

Harmonic, dashboard, migration export, security, CFG/PDG, and discover-time taint are **opt-in** via the flags below.

### Deeper analysis (opt-in)

| Flag | What it adds |
|------|----------------|
| `--with-security` | Secret scanning |
| `--with-cfg` | Per-function CFG, dominators, PDG (archive under `.rbuilder/analysis/`) |
| `--with-taint` | Discover-time taint into archive (implies CFG/PDG pass) |
| `--with-harmonic` | Harmonic centrality (migration ranking) |
| `--with-dashboard` | Static dashboard bundle under `.rbuilder/dashboard/` |
| `--export-migration-hints` | Migration roadmap JSON (alias: `--export-migration-plan`) |

```bash
# CFG so inspect / slice have rich PDG context
rbuilder discover . -l java -e target --with-cfg

# Full walkthrough set used for the samples below
rbuilder discover . -l java -e target \
  --with-cfg --with-dashboard --with-harmonic --export-migration-hints
```

Example lines from that richer run:

```text
[!] Deep analysis enabled (--with-cfg / --with-taint).
✓ Control flow analysis:
  CFG/PDG/Dominance: 178 functions analyzed
  Skipped: 9 functions (unsupported language or parse error)
[✓] Migration plan (Hybrid Default): 9 steps → …/ecommerce-java/./.rbuilder/migration_plan.json
[✓] Dashboard: …/ecommerce-java/./.rbuilder/dashboard/index.html
```

Use `--with-cfg` when you need `inspect` / slice overlays; add `--with-taint` for discover-time taint flows. On large monorepos (100k+ functions) expect minutes to hours.

### Verbose logging and stage profiling

```bash
rbuilder discover . -v
```

With `-v`, discover emits a **`[profile] discover summary`** line (wall time, peak RSS, node count) and per-stage timings.

```bash
RUST_LOG=info,profile=info rbuilder discover . --with-cfg -v -l java -e target 2>&1 \
  | tee discover-profile.log
grep '\[profile\]' discover-profile.log
```

Example profile lines (ecommerce-java, `--with-cfg`):

```text
[profile] discover summary wall_secs=0.14 index_secs=0.01 post_index_secs=0.09 \
  peak_rss_mb=27.0 functions=187 nodes=518 cfg=true security=false
[profile] stage stage="cfg_total" secs=0.030 pct_wall=21.0
[profile] stage stage="save_dashboard" secs=0.028 pct_wall=19.6
[profile] stage stage="index_extract" secs=0.012 pct_wall=8.1
```

Harmonic centrality is **off by default** — pass `--with-harmonic` when you need it for migration ranking. On kernel-scale graphs it adds ~30s wall and multi‑GB peak RSS.

See [analysis-architecture.md](analysis-architecture.md) and [internal/temp.md](internal/temp.md) for large-graph adaptive gating.

### Legacy JSON graph (optional)

By default, rBuilder writes a **binary snapshot** (`graph.snapshot.bin`). Legacy `graph.db` / `graph.json` are only written when requested:

```bash
rbuilder discover . --write-json-graph
```

### What `discover` creates

After a successful run:

```
ecommerce-java/.rbuilder/
├── graph.snapshot.bin          # Columnar mmap graph (primary cache for queries)
├── blast_engine.snapshot.bin   # Pre-built blast-radius engine
├── macro_call_index.db         # Blast-radius lookup cache (SQLite; not the graph)
├── macro_call_index.bin        # Same index in bincode (companion to .db)
├── analysis_results.bin        # Columnar analysis properties
├── file_hashes.json            # Incremental file tracker
├── migration_plan.json         # With --export-migration-hints
├── analysis/                   # Per-function CFG/PDG/taint (with --with-cfg / --with-taint)
│   └── cfg_pdg.archive.bin
└── dashboard/                  # Only with --with-dashboard
    ├── index.html
    ├── manifest.json
    ├── migration_plan.json
    └── graph_payload.bin
```

Query commands read `graph.snapshot.bin` when present. You do **not** need `graph.db` for normal CLI use.

Point every subsequent command at this repo:

```bash
export REPO="$PWD"
# or pass -r on each command:
rbuilder -r "$REPO" gql 'MATCH (n:Function) RETURN n LIMIT 5'
```

---

## 5. Global CLI flags

These apply to **every** subcommand:

| Flag | Purpose |
|------|---------|
| `-r, --repo PATH` | Repository root (default: current directory) |
| `-d, --db PATH` | Legacy graph JSON path (default: `.rbuilder/graph.db`) |
| `-f, --format FORMAT` | Output: `text`, `json`, `graphviz`, `mermaid` |
| `-o, --output FILE` | Write command output to a file instead of stdout |

Examples:

```bash
# JSON for scripting
rbuilder -r "$REPO" -f json gql 'MATCH (n:Class) RETURN n LIMIT 10'

# Mermaid diagram to a file
rbuilder -r "$REPO" -f mermaid -o checkout-cfg.mmd inspect checkout cfg
```

---

## 6. Query the graph with GQL

`gql` runs the graph query language against the indexed graph. **Run `discover` first.**

### Inventory macros

```bash
rbuilder -r "$REPO" gql --macro-name all_functions unused
```

Text mode prints one function name per line (187 on this fixture). JSON is better for scripts:

```bash
rbuilder -r "$REPO" -f json gql --macro-name all_functions unused | jq '.count'
```

```text
187
```

### Exact name match

```bash
rbuilder -r "$REPO" gql \
  "MATCH (n:Function) WHERE n.name = 'clearCart' RETURN n"
```

```text
clearCart
clearCart
```

(There are two `clearCart` methods — service and controller.)

JSON shows file paths:

```bash
rbuilder -r "$REPO" -f json gql \
  "MATCH (n:Function) WHERE n.name = 'clearCart' RETURN n" | jq '.rows'
```

```json
[
  [
    {
      "binding": "n",
      "file": "…/service/CartService.java",
      "node": "clearCart",
      "type": "Function"
    }
  ],
  [
    {
      "binding": "n",
      "file": "…/controller/CartController.java",
      "node": "clearCart",
      "type": "Function"
    }
  ]
]
```

### Classes

```bash
rbuilder -r "$REPO" -f json gql \
  "MATCH (n:Class) WHERE n.name = 'CartService' RETURN n" | jq '.rows[0]'
```

```json
[
  {
    "binding": "n",
    "file": "…/service/CartService.java",
    "node": "CartService",
    "type": "Class"
  }
]
```

### Call relationships

Who calls `clearCart`?

```bash
rbuilder -r "$REPO" gql \
  "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE b.name = 'clearCart' RETURN a,b"
```

```text
checkout -> clearCart
clearCart -> clearCart
```

JSON (trimmed):

```json
{
  "count": 2,
  "rows": [
    [
      { "binding": "a", "node": "checkout", "file": "…/OrderService.java", "type": "Function" },
      { "binding": "b", "node": "clearCart", "file": "…/CartService.java", "type": "Function" }
    ]
  ],
  "schema_version": 1
}
```

### Common node / edge types

- Nodes: `Function`, `Class`, `Interface`, `Module`, `File`, `Import`, `ConfigKey`, …
- Edges: `CALLS`, `IMPORTS`, `CONTAINS`, `DEPENDS_ON`, `IMPLEMENTS`, …

### Named communities (analysis overlay)

`discover` runs label-propagation community detection and stores assignments in
`.rbuilder/analysis_results.bin` — **not** as edges in the topology graph.
`gql` joins that sidecar so you can list and filter communities:

```bash
# Macro: list communities (id, heuristic label, member_count)
rbuilder -r "$REPO" -f json gql --macro-name all_communities unused | jq '.rows[:3]'
```

```json
[
  [
    {
      "binding": "c",
      "node": "ecommerce.service::checkout",
      "type": "Community",
      "label": "ecommerce.service::checkout",
      "community_id": 385,
      "member_count": 19,
      "file": null
    }
  ]
]
```

```bash
# Members of one community (use an id from the list above)
rbuilder -r "$REPO" -f json gql \
  "MATCH (f:Function) WHERE f.community_id = '385' RETURN f LIMIT 10" | jq '.count'

# CLI helpers (same labels; --write refreshes analysis_results.bin)
rbuilder -r "$REPO" communities list
rbuilder -r "$REPO" communities label --write
```

Labels are **heuristic** (package path, top PageRank symbol, token majority, infrastructure hubs).
They are for orientation — not ground-truth domain names. See
[community query & naming plan](design/community-query-and-naming-plan.md).

Virtual type `:Community` is query-only; there is no `MEMBER_OF` edge in the snapshot.

---

## 7. Blast radius (change impact)

`blast-radius` answers: **“What breaks upstream if I change this symbol?”**

Bare names are often ambiguous. Prefer **FQN** (`Class::method`):

```bash
rbuilder -r "$REPO" blast-radius 'CartService::clearCart'
```

```text
Blast radius for 'CartService::clearCart'
  Score: 25.1/100
  Direct callers: 1
  Impact zone: 1
  Callers: OrderService.checkout
  Impact: OrderService.checkout
```

Ambiguous bare name shows remediation:

```bash
rbuilder -r "$REPO" blast-radius clearCart
```

```text
Error: Symbol 'clearCart' is ambiguous. Found 2 matches.
UUID                                   | Class Context  | Source File Path
…                                      | CartService    | …/CartService.java
…                                      | CartController | …/CartController.java

Remediation: Refine your search query using a fully qualified namespace syntax:
  rbuilder blast-radius "ClassName::clearCart"
  rbuilder blast-radius "path/to/file.java::clearCart"
```

### Symbol forms

| Form | Example |
|------|---------|
| Bare name | `checkout` (fails if ambiguous) |
| FQN | `CartService::clearCart` |
| UUID | node id from GQL / blast JSON |

Disambiguate with filters:

```bash
rbuilder -r "$REPO" blast-radius clearCart --class CartService
rbuilder -r "$REPO" blast-radius clearCart \
  --file src/main/java/com/example/ecommerce/service/CartService.java
```

### Limit caller depth

```bash
rbuilder -r "$REPO" blast-radius 'CartService::clearCart' --depth 1
rbuilder -r "$REPO" blast-radius 'CartService::clearCart' --depth 5
```

Omit `--depth` for full transitive upstream closure.

### JSON output

```bash
rbuilder -r "$REPO" -f json blast-radius 'CartService::clearCart' \
  | jq '{score: .metrics.score, callers: .topology.direct_callers}'
```

```json
{
  "score": 25.05,
  "callers": [
    {
      "file_path": "…/OrderService.java",
      "fqn": "OrderService.checkout",
      "id": "…"
    }
  ]
}
```

Schema: [cli-output-schemas.md](cli-output-schemas.md) §1 and [json-api.md](json-api.md) §6.

### Statement-level slice hand-offs (slow)

```bash
rbuilder -r "$REPO" blast-radius 'CartService::clearCart' --with-slices
```

Requires `discover --with-cfg` for rich PDG context.

---

## 8. Program slicing and taint

`slice` performs **line-level** backward or forward slicing on a source file. Paths may be absolute, cwd-relative, or relative to `--repo`. Run `discover --with-cfg` first so PDG data is available.

### Backward slice

“What code influences this variable at this line?” — in `OrderService.checkout`, `cart` is assigned on line 52:

```bash
rbuilder -r "$REPO" slice \
  src/main/java/com/example/ecommerce/service/OrderService.java \
  --line 52 \
  --variable cart \
  --function checkout
```

```text
Backward slice for src/main/java/com/example/ecommerce/service/OrderService.java:52 (variable: cart)
Reduction: 92.3%
  52
```

A denser example from `CartService.addItem` (line 53, local `item`):

```bash
rbuilder -r "$REPO" slice \
  src/main/java/com/example/ecommerce/service/CartService.java \
  --line 53 \
  --variable item \
  --function addItem
```

```text
Backward slice for src/main/java/com/example/ecommerce/service/CartService.java:53 (variable: item)
Reduction: 92.9%
  53
```

### Forward slice

```bash
rbuilder -r "$REPO" slice \
  src/main/java/com/example/ecommerce/service/CartService.java \
  --line 38 \
  --variable cart \
  --function addItem \
  --direction forward
```

### Taint trace

```bash
rbuilder -r "$REPO" slice \
  src/main/java/com/example/ecommerce/service/OrderService.java \
  --line 83 \
  --variable cartService \
  --function checkout \
  --taint
```

### View modes

| `--view` | Description |
|----------|-------------|
| `text` | Summary (default) |
| `cfg` | CFG overlay — use with `-f mermaid` or `-f graphviz` |
| `pdg` | PDG overlay |

```bash
rbuilder -r "$REPO" -f mermaid slice \
  src/main/java/com/example/ecommerce/service/CartService.java \
  --line 53 --variable item --function addItem --view cfg
```

### `--function` names

`--function` must be the **method/function name** in the source file (as parsed by tree-sitter), not the enclosing class name:

```bash
rbuilder -r "$REPO" gql \
  "MATCH (n:Function) WHERE n.name = 'checkout' RETURN n"
```

---

## 9. Inspect CFG / PDG / dominance

`inspect` dumps semantic layers for an **indexed function symbol** (no `--class` flag — use a unique symbol or GQL to pick the right function). Run `discover --with-cfg` first.

```bash
rbuilder -r "$REPO" inspect checkout cfg
```

```text
CFG for checkout: 5 blocks, 5 edges
```

```bash
rbuilder -r "$REPO" -f json inspect checkout cfg | jq '{layer, blocks: (.nodes|length), edges: (.edges|length)}'
```

```json
{
  "layer": "cfg",
  "blocks": 5,
  "edges": 5
}
```

Mermaid CFG:

```bash
rbuilder -r "$REPO" -f mermaid inspect checkout cfg
```

```text
flowchart TD
  462c1054-… --> 14712608-…
  462c1054-… --> ae5a5a76-…
  14712608-… --> 897883b6-…
  ae5a5a76-… --> 897883b6-…
  897883b6-… --> 4165ce10-…
```

Other layers:

```bash
# Prune unreachable blocks
rbuilder -r "$REPO" inspect checkout cfg --prune

# Program dependence graph (data edges)
rbuilder -r "$REPO" inspect checkout pdg --edge-layer data
# → PDG for checkout: 13 nodes, 22 data deps, 0 control deps

rbuilder -r "$REPO" inspect checkout pdg --def-use
rbuilder -r "$REPO" inspect checkout dom --frontiers
```

---

## 10. Hybrid CPG (`cpg`)

The `cpg` façade bridges the **repo call graph** (L_repo) with the **per-function CFG/PDG archive** (L_proc) built by `discover --with-cfg`. Use it for typed field mutations, data flows, and Joern-style handoffs without stitching several CLI tools yourself.

Requires a prior `discover … --with-cfg` (the ecommerce walkthrough already uses that flag).

### Status and CALL neighborhood

```bash
rbuilder -r "$REPO" cpg status
# → CPG L_proc: ready (… functions) at …/cfg_pdg.archive.bin
# → CPG field writes: N indexed (cpg mutations)

rbuilder -r "$REPO" cpg function priceShoppingCart
rbuilder -r "$REPO" cpg calls 'ShoppingCartService::priceShoppingCart'
```

### Field mutations (CoolStore `ShoppingCart`)

Find non-constructor writes to a type — useful before converting a mutable DTO/cart model to an immutable record, or to prove pricing still mutates totals:

```bash
rbuilder -r "$REPO" cpg mutations --type ShoppingCart --exclude-ctors
```

Example (paths shortened):

```text
Mutations of ShoppingCart [excl. ctors] (7 hits):
  …/coolstore/model/ShoppingCart.java:61  this.cartTotal = cartTotal
  …/coolstore/model/ShoppingCart.java:45  this.cartItemTotal = cartItemTotal
  …
```

Pair with blast-radius on the CoolStore pricing entrypoint:

```bash
rbuilder -r "$REPO" blast-radius 'ShoppingCartService::priceShoppingCart'
# → Callers include CartEndpoint.add / delete / checkout and checkOutShoppingCart
```

**Dashboard:** after `discover --with-cfg --with-dashboard`, the **Dataflow** tab includes a **Field mutations (CPG)** panel (same filters). Click a hit to open that function’s PDG and highlight the write line. See [Dashboard user guide](dashboard-user-guide.md#dataflow).

JSON for agents:

```bash
rbuilder -r "$REPO" -f json cpg mutations --type ShoppingCart --exclude-ctors
```

Empty result means no **typed** non-ctor writes were recovered (receivers without a resolved type are omitted unless `--include-unresolved`). On C fixtures, query the struct typedef name (e.g. `shopping_cart_t`). See [agent-recipes.md](agent-recipes.md) Recipe 11 and [hybrid-cpg-plan.md](design/hybrid-cpg-plan.md).

### Flows, AST, export

```bash
# Forward flows from a variable at a line (wraps slice; optional --with-alias)
rbuilder -r "$REPO" -f json cpg flows \
  src/main/java/com/example/ecommerce/coolstore/service/ShoppingCartService.java \
  --line 75 --variable sc --function priceShoppingCart --direction forward

# Optional: discover --with-ast-skeleton then:
rbuilder -r "$REPO" -f json cpg ast priceShoppingCart

rbuilder -r "$REPO" cpg export --format graphson --output /tmp/ecommerce-cpg.json \
  --path-contains coolstore/
```

---

## 11. Graph metrics

`metrics` reports network analytics on the indexed call graph. Prefer **JSON** for scripting (text mode prints debug-style structs).

```bash
rbuilder -r "$REPO" -f json metrics --communities | jq .
```

```json
{
  "communities": {
    "assignments": 518,
    "count": 442,
    "modularity": 0.49
  },
  "schema_version": 1
}
```

That summary is counts only. For **named** communities and membership, use GQL / `communities list`
([§6](#6-query-the-graph-with-gql)) or `.rbuilder/dashboard/communities.json` after `--with-dashboard`.

```bash
rbuilder -r "$REPO" -f json metrics --pagerank | jq '.pagerank | {iterations, converged, top: .top[:3]}'
```

```json
{
  "iterations": 20,
  "converged": false,
  "top": [
    { "node": "…uuid…", "pagerank": 0.0027 },
    { "node": "…uuid…", "pagerank": 0.0015 },
    { "node": "…uuid…", "pagerank": 0.0015 }
  ]
}
```

```bash
rbuilder -r "$REPO" metrics --betweenness
rbuilder -r "$REPO" -f json metrics --pagerank --iterations 50 | jq .
```

---

## 12. Semantic search

Semantic search is **opt-in** — it does not run during `discover`. Build a separate Hamming index over function symbols, then query by natural language or keywords.

**Prerequisites:** `discover` completed. Default embedder is **code-daemon** (needs `git lfs pull` for bundled ONNX when building from source). Offline / CI: prefer `--embedder vocab` or `--embedder hash` (no ONNX).

```bash
# Build semantic index (default: code-daemon, 256-d)
rbuilder -r "$REPO" semantic index

# Incremental rebuild — reuse rows when body hash unchanged
rbuilder -r "$REPO" semantic index --incremental

# Query (JSON for agents). Late fusion is ON by default.
rbuilder -r "$REPO" -f json semantic query "shopping cart checkout" --limit 10
rbuilder -r "$REPO" -f json semantic query "OrderService" --keyword-and
# Pure Hamming (disable fusion):
rbuilder -r "$REPO" -f json semantic query "OrderService" --no-fusion --limit 10

# Community-scoped search — pool member embeddings (needs discover analysis + semantic index)
rbuilder -r "$REPO" -f json semantic query "shopping cart" --scope community --limit 5

# Hash embedder (no ONNX) — e.g. CI
rbuilder -r "$REPO" semantic index --embedder hash

# Vocab embedder (compiled token table, offline) + optional call-graph diffusion
rbuilder -r "$REPO" semantic index --embedder vocab
rbuilder -r "$REPO" semantic index --embedder vocab --diffuse \
  --diffuse-alpha 0.25 --diffuse-iters 2
```

Passing `--diffuse` recomputes dense vectors and mixes call-graph neighbors **before** sign quantization (even when `--incremental` would otherwise reuse bits). Query does not re-diffuse — restart is not required for CLI query; for the dashboard, restart `serve` after rebuilding the index.

| Flag | Purpose |
|------|---------|
| `--scope function\|community` | Rank functions (default) or pooled communities |
| `--no-fusion` | Disable late fusion (default is fusion **on**: blast, PageRank, name, token-bloom) |
| `--keyword-and` | Every query token must match metadata or body sketch |
| `--candidate-pool <N>` | Hamming pool size before fusion [default: 256] |
| `--expand neighbors\|blast\|gql\|all` | Hybrid expansion after top hits |
| `--embedder hash\|vocab\|onnx\|code-daemon` | Embedding backend [default: `code-daemon`] |
| `--dimensions <N>` | Float width before quantize; multiple of 8 [default: 256] |
| `--diffuse` / `--no-diffuse` | Jacobi call-graph mix on dense floats before quantize (index only; off by default) |
| `--diffuse-alpha` / `--diffuse-iters` | Diffusion blend weight and iterations [defaults: 0.25, 2] |
| `--diffuse-bidirectional` | Include callers as well as callees |

**Dashboard:** `rbuilder serve --open` → **Search** tab uses the same index via `/api/semantic/*`. The UI does not choose the embedder — build the index with CLI first, then restart `serve`. Status shows `model_id` (e.g. `vocab-accumulate-v1`).

**Perf note (linux-scale):** time queries with a **release** binary (`cargo build --release`). Debug builds can be ~100× slower on Hamming scan. Index load of `.rbuilder/semantic_index.bin` is bincode into owned strings (~tens of seconds at ~1.8M functions); query itself is ~few ms in release.

Design → **[Semantic search design](design/semantic-search-design.md)** · timing tests → `cargo test --test semantic_query_timing -- --nocapture`

---

## 13. Export graph projections

`export` writes the graph or a **filter-selected** subgraph to a file. The `--query` flag uses **filter syntax**, not GQL `MATCH` (all formats honor the filter, including JSON):

| Query | Meaning |
|-------|---------|
| `all` | Entire graph |
| `name:clearCart` | Nodes with exact name |
| `type:Function` | All functions |
| `functions` | Shortcut for function nodes |

```bash
rbuilder -r "$REPO" export \
  --export-format mermaid \
  --export-output cart-clear.mmd \
  --query "name:clearCart"
```

```text
Exported 2 nodes, 1 edges -> cart-clear.mmd
```

```bash
# Full graph as JSON / GraphML / DOT
rbuilder -r "$REPO" export --export-format json --export-output ecommerce-graph.json --query all
rbuilder -r "$REPO" export --export-format graphml --export-output ecommerce.graphml --query all
rbuilder -r "$REPO" export --export-format graphviz --export-output calls.dot --query all
```

For GQL pattern matching, use `rbuilder gql` — or `rbuilder serve` + [HTTP API](http-api.md).

---

## 14. CI policy check

`check` evaluates blast-radius policy rules against functions changed in the current git working tree (or all functions if git is unavailable).

Example policy files: [docs/examples/policy-strict.json](examples/policy-strict.json). Format: [policy-format.md](policy-format.md).

```bash
rbuilder -r "$REPO" check --policy-file policy.json
```

Exit code **1** when violations are found — suitable for CI pipelines.

The fixture also ships a shared policy at [`rbuilder-tests/rbuilder-policy.json`](../rbuilder-tests/rbuilder-policy.json).

---

## 15. HTTP server (`serve`)

`serve` starts a local HTTP server with the **dashboard** and **GQL query API** (default `http://127.0.0.1:8080/`). Discover with `--with-dashboard` first if you want the static UI assets.

```bash
rbuilder -r "$REPO" discover . -l java -e target --with-dashboard
rbuilder -r "$REPO" serve --open
```

| Endpoint | Purpose |
|----------|---------|
| `/` | Dashboard UI |
| `POST /api/query` | GQL / macros (JSON body) |
| `GET /api/semantic/status` | Semantic index availability |
| `POST /api/semantic/query` | Semantic search (JSON body) |
| `/api/health` | Health check |

```bash
curl -sS -X POST http://127.0.0.1:8080/api/query \
  -H 'Content-Type: application/json' \
  -d '{"macro":"all_functions"}' | jq '.count'
```

Full reference: [http-api.md](http-api.md).

### Legacy socket daemon

For blast-radius auto-connect only (no HTTP):

```bash
rbuilder -r "$REPO" serve --daemon
# Terminal 2 — auto-uses .rbuilder/query.sock when present
rbuilder -r "$REPO" -f json blast-radius 'CartService::clearCart'
```

Disable auto-connect: `RBUILDER_NO_QUERY_DAEMON=1`.

---

## 16. Recommended workflow

```bash
# 1. Point at the in-tree fixture
cd /path/to/rBuilder
export REPO="$PWD/rbuilder-tests/ecommerce-java"
cd "$REPO"

# 2. Index (add CFG + dashboard for the rest of this walkthrough)
rbuilder discover . -l java -e target \
  --with-cfg --with-dashboard --with-harmonic --export-migration-hints

# 3. Explore structure
rbuilder -r "$REPO" -f json gql --macro-name all_functions unused | jq '.count'
rbuilder -r "$REPO" -f json gql --macro-name all_communities unused | jq '.rows[:5]'
rbuilder -r "$REPO" communities list | head -15
rbuilder -r "$REPO" gql \
  "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE b.name = 'clearCart' RETURN a,b"

# 4. Change-impact before editing
rbuilder -r "$REPO" blast-radius 'CartService::clearCart'
rbuilder -r "$REPO" -f json blast-radius 'CartService::clearCart' | jq '.metrics'

# 5. CoolStore dual API + hybrid CPG (field mutations)
rbuilder -r "$REPO" cpg status
rbuilder -r "$REPO" cpg mutations --type ShoppingCart --exclude-ctors
rbuilder -r "$REPO" blast-radius 'ShoppingCartService::priceShoppingCart'

# 6. Architectural hotspots
rbuilder -r "$REPO" -f json metrics --communities | jq .
rbuilder -r "$REPO" -f json metrics --pagerank | jq '.pagerank.top[:5]'

# 7. Deep dive on checkout
rbuilder -r "$REPO" inspect checkout cfg
rbuilder -r "$REPO" slice \
  src/main/java/com/example/ecommerce/service/CartService.java \
  --line 53 --variable item --function addItem

# 8. Export / dashboard
rbuilder -r "$REPO" export --export-format mermaid \
  --export-output clearCart.mmd --query 'name:clearCart'
rbuilder -r "$REPO" serve --open
```

Migration hints (with `--export-migration-hints`) land under `.rbuilder/migration_plan.json` and `.rbuilder/dashboard/migration_plan.json` — package-level steps such as `com.example.ecommerce.service`, `…repository`, `…controller`, and CoolStore `…coolstore.*`.

---

## 17. Command reference

| Command | Purpose |
|---------|---------|
| `discover` | Index repo, build `.rbuilder/` artifacts |
| `gql` | Graph query language (incl. virtual `:Community`) |
| `communities` | List / refresh heuristic community labels |
| `blast-radius` | Upstream call-graph impact for a symbol |
| `slice` | Line-level program slice or taint trace |
| `inspect` | CFG / PDG / dominance for a function |
| `cpg` | Hybrid CPG: status, mutations, flows, calls, export (needs `--with-cfg`) |
| `metrics` | PageRank, betweenness, communities summary |
| `export` | Serialize graph (json, graphml, dot, mermaid) |
| `check` | CI policy gateway |
| `semantic` | Opt-in function semantic index + query (`--scope community`) |
| `serve` | HTTP dashboard + `/api/query` + `/api/semantic/*` (default); `serve --daemon` for blast socket |

### `discover` flags

| Flag | Description |
|------|-------------|
| `-l, --languages` | Comma-separated filter (`java`, `typescript`, `rust`, …) |
| `-e, --exclude` | Comma-separated path exclude patterns |
| `-v, --verbose` | Debug logging + stage profile lines |
| `--with-security` | Secret scanning |
| `--with-cfg` | CFG / PDG (not taint) |
| `--with-taint` | Discover-time taint (implies CFG pass) |
| `--with-harmonic` | Harmonic centrality (default off) |
| `--with-dashboard` | Static dashboard bundle (default off) |
| `--export-migration-hints` | Migration roadmap JSON |
| `--write-json-graph` | Also write legacy `graph.db` / `graph.json` |

There is no umbrella `--all` flag — combine `--with-cfg --with-security --with-taint` explicitly when you want the former deep pass.

---

## 18. Troubleshooting

### `Graph not found` / `run discover first`

```bash
rbuilder discover . -l java -e target
# or
rbuilder -r "$REPO" gql 'MATCH (n:Function) RETURN n LIMIT 1'
```

### Symbol not found / ambiguous (`blast-radius`, `inspect`)

List exact names, then use FQN:

```bash
rbuilder -r "$REPO" gql "MATCH (n:Function) WHERE n.name = 'clearCart' RETURN n"
rbuilder -r "$REPO" blast-radius 'CartService::clearCart'
rbuilder -r "$REPO" blast-radius clearCart --class CartService
```

`inspect` takes a **function** name (`checkout`, `addItem`), not a class name (`CartService`).

### Slice parse / PDG errors

Ensure you ran `discover --with-cfg`, then pass the method name and a variable that exists on that line:

```bash
rbuilder -r "$REPO" slice \
  src/main/java/com/example/ecommerce/service/CartService.java \
  --line 53 --variable item --function addItem --language java
```

### Empty `cpg mutations`

Confirm `cpg status` shows a field-write index, then match the **resolved type name** (Java/C#/…: `ShoppingCart`; C: `shopping_cart_t`). Setters count as mutation sites; unresolved receivers are omitted unless `--include-unresolved`. Re-run `discover --with-cfg` after adding CoolStore sources.

### Slow `discover`

Start with the default mode. Add `--with-cfg` or `--with-taint` only when you need inspect, slice overlays, or taint. Keep `--with-harmonic` / `--with-dashboard` off unless you need migration ranking or the static UI.

On **very large repos** (500k+ graph nodes), discover automatically:

- Caps PageRank iterations and relaxes convergence tolerance
- Caps HyperBall harmonic rounds (when `--with-harmonic`) and parallelizes propagation
- Skips per-function rows in `function_metrics.json` (community/metagraph view instead)
- Uses on-demand blast reachability for flat call graphs (no eager multi-hundred-GB bitsets)

Profile a cold run:

```bash
rm -rf .rbuilder
RUST_LOG=info,profile=info rbuilder discover . -v 2>&1 | grep '\[profile\]'
```

### Further reading

- [Introduction](Introduction.md) — concepts and feature goals
- [cli-getting-started.md](cli-getting-started.md) — shorter walkthrough
- [http-api.md](http-api.md) — dashboard HTTP API
- [json-api.md](json-api.md) / [cli-output-schemas.md](cli-output-schemas.md) — machine-readable output
- [AGENTS.md](../AGENTS.md) — agent-oriented command recipes
- [`rbuilder-tests/README.md`](../rbuilder-tests/README.md) — all language fixtures + correctness suite
