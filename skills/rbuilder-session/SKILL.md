---
name: rbuilder-session
description: >-
  Run multiple rBuilder queries in one task using the HTTP server.
  Use when the user needs interactive exploration, batch analysis,
  repeated queries, or the visual dashboard.
  Activates on: multiple queries, interactive exploration, keep querying,
  batch analysis, HTTP session, dashboard, start server, many queries.
compatibility: Requires rbuilder CLI (v0.4+). Run `rbuilder --version` to verify.
metadata:
  author: rbuilder
  version: "1.0"
---

## Prerequisites

- A `.rbuilder/` directory must exist in the repo root. If missing, run:
  ```bash
  rbuilder discover .
  ```
- For semantic search via HTTP, run `rbuilder semantic index` first.

## Decision Tree

| User intent | Command |
|-------------|---------|
| Start HTTP server | `rbuilder serve --port 8080` |
| Start query-only (no dashboard) | `rbuilder serve --port 8080 --query-only` |
| Open dashboard in browser | `rbuilder serve --port 8080 --open` |
| Graph query via HTTP | `curl -X POST http://127.0.0.1:8080/api/query -H 'Content-Type: application/json' -d '{"query": "MATCH (n:Function) RETURN n LIMIT 5"}'` |
| Run macro via HTTP | `curl -X POST http://127.0.0.1:8080/api/query -H 'Content-Type: application/json' -d '{"macro": "all_functions"}'` |
| Semantic search via HTTP | `curl -X POST http://127.0.0.1:8080/api/semantic/query -H 'Content-Type: application/json' -d '{"query": "checkout flow"}'` |
| Check semantic index status | `curl http://127.0.0.1:8080/api/semantic/status` |
| Health check | `curl http://127.0.0.1:8080/api/health` |

## Output Contract

HTTP responses return the same JSON shapes as CLI commands:

| Endpoint | Method | Request body | Response |
|----------|--------|-------------|----------|
| `/api/query` | POST | `{"query": "..."}` or `{"macro": "..."}` or `{"query": "...", "explain": true}` | Same as `gql -f json` |
| `/graphql` | POST | Same as `/api/query` (alias) | Same as `gql -f json` |
| `/api/semantic/query` | POST | `{"query": "...", "limit": 20}` | Same as `semantic query -f json` |
| `/api/semantic/status` | GET | — | `{"available": true/false, "model_id": "...", "dimensions": 256, "functions_indexed": N}` |
| `/api/health` | GET | — | Health status |

See [commands reference](references/commands.md) for full details.

## Stop Conditions

Do **not** use this skill when:
- The user needs a **single one-off query** → use the CLI directly (rbuilder-orient, rbuilder-impact, etc.)
- The user does not need a **persistent session** → other skills are simpler

## Failure Playbook

| Symptom | Fix |
|---------|-----|
| `Address already in use` | Try a different port: `--port 8081` |
| Server won't start | Check that `.rbuilder/` exists (run `discover` first) |
| Semantic endpoint returns `available: false` | Run `rbuilder semantic index` then restart `serve` |
| `--daemon` conflicts error | `--daemon` is legacy and conflicts with `--port`/`--open` — use `serve` without `--daemon` |

## Example Turn

**User:** "I need to run several queries to understand this codebase."

**Agent:**
1. `rbuilder serve --port 8080 --query-only &` (start in background)
2. `curl -s -X POST http://127.0.0.1:8080/api/query -H 'Content-Type: application/json' -d '{"macro": "all_functions"}' | jq '.count'`
3. `curl -s -X POST http://127.0.0.1:8080/api/query -H 'Content-Type: application/json' -d '{"query": "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE a.name = 'main' RETURN a,b"}' | jq '.rows | length'`

**Reply:** "I started the query server. The repo has 1,847 functions. `main` directly calls 12 other functions. Let me drill into the hotspots next."
