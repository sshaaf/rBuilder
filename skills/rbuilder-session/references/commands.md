# rbuilder-session Command Reference

Detailed flag and JSON output reference for commands used by this skill.
For full HTTP API docs, see [docs/http-api.md](../../docs/http-api.md).

## `serve`

```bash
rbuilder serve --port 8080
rbuilder serve --port 8080 --query-only
rbuilder serve --port 8080 --open
```

| Flag | Type | Default | Notes |
|------|------|---------|-------|
| `--host` | string | `127.0.0.1` | Bind host |
| `--port` | u16 | 8080 | HTTP port |
| `--open` | bool | false | Open dashboard in browser |
| `--query-only` | bool | false | Serve query API only (no dashboard) |
| `--dashboard-only` | bool | false | Serve dashboard only (no query API) |
| `--dashboard-dir` | path | `.rbuilder/dashboard` | Dashboard directory |
| `--daemon` | bool | false | Legacy socket daemon mode (conflicts with `--host`, `--port`, `--open`, `--query-only`, `--dashboard-only`, `--dashboard-dir`) |

## HTTP endpoints

### `POST /api/query`

Request body:
```json
{"query": "MATCH (n:Function) RETURN n LIMIT 5"}
```
or:
```json
{"macro": "all_functions"}
```
or:
```json
{"query": "MATCH (n:Function) RETURN n", "explain": true}
```

All three fields are optional but at least `query` or `macro` must be provided (returns HTTP 400 otherwise).

Response: same as `gql -f json`.

### `POST /graphql`

Alias for `/api/query`. Same request/response.

### `POST /api/semantic/query`

Request body:
```json
{
  "query": "checkout flow",
  "limit": 20,
  "fusion": true,
  "candidate_pool": 256,
  "keyword_and": false,
  "expand": "neighbors",
  "expand_depth": 1
}
```

Only `query` is required. All other fields have defaults.

### `GET /api/semantic/status`

Response:
```json
{
  "available": true,
  "model_id": "code-daemon",
  "dimensions": 256,
  "functions_indexed": 1847,
  "graph_digest": "abc123..."
}
```

When unavailable: `{"available": false, "message": "Semantic index not found ..."}`.

### `GET /api/health`

Returns health status.
