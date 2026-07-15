# Dashboard user guide

Interactive browser UI for exploring a repository after `discover`. This guide is for **end users**; engineering detail lives in [dashboard-design.md](dashboard-design.md).

**CLI equivalents:** each tab’s **Query Guide** panel lists matching `rbuilder` commands.

---

## Prerequisites

1. Index the repo:

```bash
cd /path/to/your/repo
rbuilder discover .          # fast graph + dashboard
# or
rbuilder discover . --all    # CFG, PDG, taint, migration export
```

2. Open the dashboard over **HTTP** (required for WASM):

```bash
# Option A — integrated server (dashboard + query API)
rbuilder serve --open

# Option B — static files only
cd .rbuilder/dashboard && python3 -m http.server 8765
# open http://localhost:8765/
```

Do **not** open `index.html` via `file://` — the graph worker cannot load `graph_payload.bin`.

---

## Layout

| Area | Description |
|------|-------------|
| **Stat cards** | Node/edge/function counts from `manifest.json` |
| **Tab bar** | Graph, Search, Functions, CFG, Dataflow, Slice, Blast, Taint, Migration, Query Guide |
| **Tab panels** | Collapsible help text per tab (click header to expand) |
| **Notification menu** | Engine/WASM status, manifest errors |

Screenshot placeholders (capture with `dashboard/scripts/capture-migration-screenshots.mjs` pattern → `docs/images/dashboard/`):

- `dash-overview.png` — full shell with stat cards
- `dash-query-guide.png` — Query Guide tab

---

## Tab guide

### Search

- Natural-language and keyword search over indexed functions (code-daemon embeddings by default).
- **Late fusion** blends Hamming similarity with blast score, PageRank, name overlap, and token-bloom sketches.
- Requires `rbuilder semantic index` and **`rbuilder serve`** (HTTP API at `/api/semantic/*` — not static-only hosting).
- **CLI:** `semantic index`, `semantic query "…"`, `--keyword-and`, `--expand neighbors`

### Graph

- **Package metagraph** — zoomable WebGL view of communities / packages.
- **Drill-down** — click a package node to expand member functions (WASM `expand`).
- **Filters** — search box, community filter, function/class type mask.
- **CLI:** `gql`, `export`, `metrics --communities`

### Functions

- Sortable table: PageRank, betweenness, harmonic, blast score.
- WASM paginated list over the full function inventory.
- **CLI:** `gql --macro-name all_functions`, `metrics --pagerank`

### CFG

- Pick a function from the list; view control-flow blocks and dominance.
- **Large repos:** when per-function JSON is omitted (`archive_only`), a banner offers **Load CFG graph** — fetches one function from the CFG record pack on demand.
- **CLI:** `inspect <symbol> cfg`, `inspect <symbol> dom --frontiers`

### Dataflow

- PDG visualization and statement list; dominator tree mode.
- **CLI:** `inspect <symbol> pdg`, `slice ... --view pdg`

### Slice

- Enter file path, line, variable, direction; highlights affected lines.
- Requires `discover --cfg` / `--all` and exported slice bundles.
- **CLI:** `slice <file> --line N --variable V --function <methodName>`

### Blast radius

- Summary cards use full-graph blast scores from discover.
- Caller table respects the **depth slider** (may differ from sidebar score).
- **CLI:** `blast-radius <symbol> --depth N`

### Taint

- Lists source→sink flows exported at discover time.
- **CLI:** `slice ... --taint` for on-demand trace at a line

### Migration

- Tune α/β/γ weights and presets; package graph + ordered table.
- Requires `discover --all --export-migration-plan`.
- Screenshots: [design/README.md](design/README.md) (figures under `docs/images/design/`).
- **CLI:** `discover . --all --export-migration-plan`

### Query Guide

- Scrollable **CLI cookbook** organized by tab (prerequisites, commands, notes).
- Validated against gbuilder: `dashboard/scripts/validate-guide-cli-gbuilder.sh`
- Live GQL in the browser requires `rbuilder serve` ([HTTP API](http-api.md)).

---

## Large repositories

| Symptom | Cause | Action |
|---------|-------|--------|
| CFG tab shows warning, no graph | `archive_only` mode (too many functions for inline JSON) | Click **Load CFG graph** per function |
| Slow first tab load | Large `graph_payload.bin` | Normal; WASM parses columnar snapshot once |
| Blank graph | Served over `file://` | Use `python3 -m http.server` or `rbuilder serve` |

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| “Graph not found” / empty stats | Run `rbuilder discover .` in repo root |
| WASM engine error in notifications | Rebuild dashboard (`npm run build` in `dashboard/`) and re-run `discover` |
| Stale data after git pull | Re-run `discover` |
| Semantic search empty / warning | Index not built or served without API | `rbuilder semantic index` then `rbuilder serve --open` |
| Migration tab empty | `discover . --all --export-migration-plan` |

---

## See also

- [Introduction — Dashboard](Introduction.md#dashboard-visual-exploration)
- [User Guide](user-guide.md)
- [HTTP API](http-api.md) — `rbuilder serve` query endpoint
