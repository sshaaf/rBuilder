# HTTP query API (`rbuilder serve`)

`rbuilder serve` starts a local HTTP server that serves the **static dashboard** and a **GQL query API** on the same origin.

**CLI reference:** [User Guide §15](user-guide.md#15-http-server-serve)

---

## Default behavior

```bash
rbuilder -r "$REPO" discover .
rbuilder -r "$REPO" serve
```

| URL | Purpose |
|-----|---------|
| `http://127.0.0.1:8080/` | Dashboard (`index.html`) |
| `http://127.0.0.1:8080/api/query` | GQL / macro queries (POST JSON) |
| `http://127.0.0.1:8080/graphql` | Alias for `/api/query` |
| `http://127.0.0.1:8080/api/health` | Health check (GET) |
| `http://127.0.0.1:8080/api/semantic/status` | Semantic index status (GET) |
| `http://127.0.0.1:8080/api/semantic/query` | Semantic search (POST JSON) |

Open browser automatically:

```bash
rbuilder -r "$REPO" serve --open
```

### Options

| Flag | Effect |
|------|--------|
| `--host`, `--port` | Bind address (default `127.0.0.1:8080`) |
| `--dashboard-dir DIR` | Override `.rbuilder/dashboard` |
| `--query-only` | API only, no static files |
| `--dashboard-only` | Dashboard only, no query API |
| `--daemon` | **Legacy** Unix-socket blast daemon (no HTTP) |

---

## Query API

### Request

`POST /api/query` with `Content-Type: application/json`

**GQL query:**

```json
{
  "query": "MATCH (n:Function) WHERE n.name LIKE '*Service*' RETURN n LIMIT 10"
}
```

**Macro:**

```json
{
  "macro": "all_functions"
}
```

**Explain plan:**

```json
{
  "query": "MATCH (n:Function) RETURN n LIMIT 5",
  "explain": true
}
```

### curl example

```bash
curl -sS -X POST http://127.0.0.1:8080/api/query \
  -H 'Content-Type: application/json' \
  -d '{"macro":"all_functions"}' | jq '.count'

curl -sS -X POST http://127.0.0.1:8080/api/query \
  -H 'Content-Type: application/json' \
  -d '{"macro":"all_communities"}' | jq '.rows[:5]'
```

`serve` loads `.rbuilder/analysis_results.bin` so virtual `:Community` nodes and `community_id` filters work the same as CLI `gql`.

### Response

Same JSON shape as `rbuilder -f json gql` on the CLI. See [json-api.md](json-api.md) §5.

Errors return HTTP 400 with a plain-text message body.

---

## Semantic search API

Requires `rbuilder semantic index` before `serve` (embedder chosen at index time: `code-daemon` default, or `vocab` / `hash` / `onnx`). Restart `serve` after rebuilding `.rbuilder/semantic_index.bin`. Same origin as the dashboard.

### `GET /api/semantic/status`

Returns JSON: `{ "available": true, "model_id": "...", "dimensions": N, "functions_indexed": N }` when the index loaded (`model_id` may be `code-daemon:v1`, `vocab-accumulate-v1`, `sign-hash-v1`, …).

### `POST /api/semantic/query`

`Content-Type: application/json`

```json
{
  "query": "shopping cart checkout",
  "limit": 20,
  "fusion": true,
  "keyword_and": false,
  "scope": "function"
}
```

`scope` may be `"function"` (default) or `"community"` (pooled member embeddings; requires discover analysis).

Response matches `rbuilder -f json semantic query`. Errors return HTTP 503 when the index is missing.

```bash
curl -sS http://127.0.0.1:8080/api/semantic/status | jq .
curl -sS -X POST http://127.0.0.1:8080/api/semantic/query \
  -H 'Content-Type: application/json' \
  -d '{"query":"OrderService","limit":5}' | jq '.hits[:3]'
curl -sS -X POST http://127.0.0.1:8080/api/semantic/query \
  -H 'Content-Type: application/json' \
  -d '{"query":"checkout","scope":"community","limit":5}' | jq '.hits'
```

---

## Serving dashboard without the API

Static hosting (no Rust process after export):

```bash
cd .rbuilder/dashboard && python3 -m http.server 8765
# open http://localhost:8765/
```

WASM requires HTTP (not `file://`). The in-browser worker cannot run full GQL — use `rbuilder serve` for live queries or the CLI.

---

## Legacy socket daemon

For backward compatibility only:

```bash
rbuilder -r "$REPO" serve --daemon
rbuilder -r "$REPO" serve --daemon --socket /tmp/rbuilder.sock --idle-secs 600
```

Subsequent `blast-radius` commands may auto-connect to `.rbuilder/query.sock` unless `RBUILDER_NO_QUERY_DAEMON=1`.

---

## Not exposed over HTTP

These CLI surfaces are **not** available as HTTP routes today (use `-f json` on the CLI instead):

- `blast-radius`, `metrics`, `check`, `slice`, `inspect`
- `communities`, `cpg`, `export`
- `discover` (indexing remains a local CLI operation)

**Exposed today:** `POST /api/query` (GQL), `GET/POST /api/semantic/*` (see above), plus the static dashboard UI.

---

## See also

- [AGENTS.md](../AGENTS.md) — agent integration patterns
- [Dashboard user guide](dashboard-user-guide.md) — browser UI
