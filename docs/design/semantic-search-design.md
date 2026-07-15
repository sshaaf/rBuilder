# Semantic Search — Engineering Design

**Opt-in natural-language and keyword search** over indexed function symbols: binary-quantized code embeddings (default **code-daemon**), Hamming retrieval, optional **late fusion** with blast radius, PageRank, name overlap, and eager **token-bloom** sketches from discover.

![Search tab — query form and fusion-ranked results (gbuilder)](../images/design/semantic-search/semantic-search-results.png)

*Figure 1: Dashboard **Search** tab — Ready badge, query controls (late fusion, keyword AND), and ranked hit table.*

---

## 1. Goals

| Goal | How |
|------|-----|
| Find functions by intent, not exact names | code-daemon ONNX embeddings (256-d MRL default) |
| Keep discover lean | Separate `.rbuilder/semantic_index.bin` — built via `semantic index` |
| Fast retrieval at scale | Sign-quantized vectors + Hamming top-k |
| Blend structure + semantics | Late fusion re-ranks Hamming pool with graph signals |
| Agent-ready output | `-f json semantic query` + HTTP `/api/semantic/query` |
| Incremental rebuilds | Reuse rows when `code_hash` unchanged |

---

## 2. Architecture overview

```mermaid
flowchart TB
  subgraph discover["discover"]
    G[graph.snapshot.bin]
    AR[analysis_results.bin]
    SK[token bloom sketches per function]
    G --> AR
    G --> SK
  end

  subgraph semantic_index["semantic index (opt-in)"]
    EX[semantic_extract body tokens]
    EM[code-daemon / hash / custom ONNX]
    BIN[semantic_index.bin Hamming rows]
    EX --> EM --> BIN
  end

  subgraph query["semantic query"]
    H[Hamming candidate pool]
    F[late fusion re-rank]
    HY[optional hybrid expand]
    H --> F --> HY
  end

  discover --> semantic_index
  BIN --> query
  AR --> F
  SK --> F
```

**Two-stage retrieval** (`semantic_fusion.rs`): Hamming pre-filter (default pool 256) → weighted fusion of semantic, blast, centrality, name, and sketch scores.

---

## 3. Embedders

| Embedder | CLI | Notes |
|----------|-----|-------|
| **code-daemon** (default) | `--embedder code-daemon` | Bundled ONNX + SentencePiece in `rbuilder-analysis/assets/`; requires `semantic-onnx` feature (default) |
| **sign-hash** | `--embedder hash` | Deterministic, no model files — CI / `--no-default-features` |
| **custom ONNX** | `--embedder onnx --model PATH` | Optional `--tokenizer` for SentencePiece |

Default dimensions: **256** (MRL for code-daemon). Clone needs `git lfs pull` for ~206 MB external weights.

Escape hatch for builds without ONNX:

```bash
cargo build --release --no-default-features
rbuilder semantic index --embedder hash
```

---

## 4. Structural sketches (Phase A)

At discover/extract time, each function gets a **256-bit token bloom** (`structural_sketch.rs`) over declaration + body tokens. Sketches are stored on graph nodes and used for:

- **Keyword AND** filter — every query token must hit metadata or sketch
- **Fusion term** — Jaccard-style overlap between query tokens and sketch

No extra index pass required beyond normal `discover`.

---

## 5. Rust implementation map

| Component | Path |
|-----------|------|
| Index + Hamming search | `crates/rbuilder-analysis/src/semantic_search.rs` |
| Body token extraction | `crates/rbuilder-analysis/src/semantic_extract.rs` |
| Late fusion | `crates/rbuilder-analysis/src/semantic_fusion.rs` |
| Hybrid expansion | `crates/rbuilder-analysis/src/semantic_hybrid.rs` |
| Bundled code-daemon | `crates/rbuilder-analysis/src/semantic_embedded.rs` |
| ONNX runtime path | `crates/rbuilder-analysis/src/semantic_onnx.rs` |
| Token bloom at extract | `crates/rbuilder-analysis/src/structural_sketch.rs` |
| CLI | `src/cli/semantic.rs`, `semantic_output.rs` |
| HTTP API | `src/cli/semantic_api.rs`, `http_serve.rs` |
| Manifest export | `crates/rbuilder-export/src/manifest.rs` |

---

## 6. Dashboard implementation

| Piece | Path |
|-------|------|
| Tab | `dashboard/src/SearchView.tsx` |
| HTTP client | `dashboard/src/semanticSearch.ts` |
| Status + query API | `GET /api/semantic/status`, `POST /api/semantic/query` |

Requires `rbuilder serve` (not static `python -m http.server`) so the semantic API is available.

---

## 7. CLI usage

```bash
rbuilder discover .
rbuilder semantic index                    # default code-daemon, 256-d
rbuilder semantic index --incremental      # reuse unchanged code_hash rows
rbuilder -f json semantic query "shopping cart checkout" --limit 10
rbuilder -f json semantic query "OrderService" --keyword-and --fusion
rbuilder -f json semantic query "auth login" --expand neighbors --expand-depth 2

rbuilder serve --open   # dashboard Search tab + /api/semantic/*
```

Index-only flags: `--embedder`, `--dimensions`, `--model`, `--tokenizer`, `--incremental`.

Query flags: `--fusion`, `--no-fusion`, `--keyword-and`, `--candidate-pool`, `--expand`, `--expand-depth`.

---

## 8. On-disk artifacts

| Path | Content |
|------|---------|
| `.rbuilder/semantic_index.bin` | Quantized embeddings + metadata (schema v2) |
| `.rbuilder/dashboard/manifest.json` | `semantic` section when index present |

---

## 9. Testing

| Layer | Location |
|-------|----------|
| Hamming + index roundtrip | `crates/rbuilder-analysis/src/semantic_search.rs` tests |
| Fusion scoring | `crates/rbuilder-analysis/src/semantic_fusion.rs` tests |
| CLI subprocess | `tests/cli_output/subprocess_golden_path.rs` |
| HTTP semantic API | `src/cli/http_serve.rs` unit tests |

Regenerate screenshots:

```bash
rbuilder -r ~/git/java/gbuilder semantic index
rbuilder -r ~/git/java/gbuilder serve --port 8080
DASHBOARD_URL=http://127.0.0.1:8080/ node dashboard/scripts/capture-design-screenshots.mjs
```

Demo video (5 s per feature, tab + panel highlighted):

```bash
DASHBOARD_URL=http://127.0.0.1:8080/ node dashboard/scripts/record-feature-demo.mjs
```

---

## 10. Related docs

- [Blast radius design](blast-radius-design.md) — fusion blast term + hybrid `--expand blast`
- [Graph metrics design](graph-metrics-design.md) — PageRank centrality term
- [HTTP API](../http-api.md) — `/api/semantic/*`
- [CLI output schemas](../cli-output-schemas.md) — semantic JSON shapes
