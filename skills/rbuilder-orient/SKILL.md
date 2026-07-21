---
name: rbuilder-orient
description: >-
  Explore an unfamiliar repository using rBuilder's code knowledge graph.
  Use when the user asks to understand codebase structure, find important
  functions, get a repo overview, or explore what a project does.
  Activates on: unfamiliar repo, explore structure, understand codebase,
  what does this project do, find important functions, repo overview,
  codebase map, architecture overview.
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
- For natural-language search, run `rbuilder semantic index` separately after discover — it is a distinct indexing step that builds the embedding index.

## Decision Tree

| User intent | Command |
|-------------|---------|
| Repo overview (files, nodes, edges) | `rbuilder discover -f json` |
| Key functions by importance | `rbuilder metrics --pagerank -f json` |
| List all functions | `rbuilder gql --macro-name all_functions unused -f json` |
| Call graph overview | `rbuilder gql --macro-name call_chain unused -f json` |
| Find functions by name pattern | `rbuilder gql "MATCH (n:Function) WHERE n.name LIKE '*Handler' RETURN n LIMIT 20" -f json` |
| Natural-language search | `rbuilder semantic query "authentication logic" -f json` |

> **Note:** `--macro-name` requires a positional query argument even though it is ignored. Pass `unused` as a placeholder.

## Output Contract

Always use `-f json`. Key fields to surface:

| Command | Key fields |
|---------|-----------|
| `discover` | `.metrics.files_discovered`, `.metrics.files_indexed`, `.metrics.nodes_generated`, `.metrics.edges_generated`, `.metrics.duration_ms` |
| `metrics --pagerank` | `.pagerank.top[]` — each entry has `.node` (UUID) and `.pagerank` (score). Report top 10. |
| `gql` | `.rows[]` — each row is an array of bindings with `.binding`, `.node`, `.type`, `.file` |
| `semantic query` | `.hits[]` — each hit has function name, file, and relevance score |

See [commands reference](references/commands.md) for full JSON shapes.

## Stop Conditions

Do **not** use this skill when:
- The user already knows the repo structure and is asking about a **specific symbol** → use **rbuilder-impact** instead
- The user wants to understand **data flow within a function** → use **rbuilder-slice** instead
- The user wants **change impact analysis** before editing → use **rbuilder-impact** instead

## Failure Playbook

| Symptom | Fix |
|---------|-----|
| `Error: graph snapshot not found` | Run `rbuilder discover .` first |
| `query macro not found` | Check macro name — available macros: `all_functions`, `direct_calls`, `call_chain` |
| `semantic index not found` | Run `rbuilder semantic index` (separate step from discover) |
| Too many results | Add `LIMIT N` to GQL queries |

## Example Turn

**User:** "I just cloned this repo. What's the architecture?"

**Agent:**
1. `rbuilder discover . -f json` → get file/node/edge counts
2. `rbuilder metrics --pagerank -f json | jq '.pagerank.top[:10]'` → find architectural hotspots
3. `rbuilder gql --macro-name all_functions unused -f json | jq '.count'` → total function count

**Reply:** "This repo has 342 files with 1,847 functions and 5,203 call edges. The top PageRank functions are `ShoppingCartService.checkout` (0.034), `OrderProcessor.validate` (0.028), and `PaymentGateway.charge` (0.025) — these are the architectural hotspots."
