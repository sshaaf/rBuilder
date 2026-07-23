---
name: rbuilder-neighborhood
description: >-
  Explore call chains, dependency graphs, and function relationships.
  Export subgraphs to Mermaid, GraphML, or Graphviz.
  Use when the user asks about call chains, what calls what, function
  dependencies, or needs to export a subgraph for visualization.
  Activates on: call chain, dependency graph, what calls what, function
  relationships, export subgraph, visualize dependencies, call tree.
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

## Decision Tree

| User intent | Command |
|-------------|---------|
| Direct callees of X | `rbuilder gql "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE a.name = 'X' RETURN a,b" -f json` |
| Direct callers of X | `rbuilder gql "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE b.name = 'X' RETURN a,b" -f json` |
| N-hop call neighborhood | `rbuilder gql "MATCH (a:Function)-[:CALLS*1..N]->(b:Function) WHERE a.name = 'X' RETURN a,b LIMIT 50" -f json` |
| Built-in call chain macro | `rbuilder gql --macro-name call_chain unused -f json` |
| Export as Mermaid | `rbuilder export --export-format mermaid --export-output graph.mmd --query "name:X"` |
| Export as GraphML | `rbuilder export --export-format graphml --export-output graph.graphml --query all` |
| Export as Graphviz | `rbuilder export --export-format graphviz --export-output graph.dot --query "type:Function"` |
| Export as JSON | `rbuilder export --export-format json --export-output graph.json --query functions` |

> **Note:** `export --query` uses **filter syntax** (`all`, `name:Foo`, `type:Function`, `functions`, `classes`, compound `|`), **not** GQL MATCH syntax. The `name:` filter uses exact match.

## Output Contract

Always use `-f json` for `gql`. Export writes directly to the file specified by `--export-output`.

| Command | Key fields |
|---------|-----------|
| `gql` | `.rows[]` — each row is an array of bindings with `.binding`, `.node`, `.type`, `.file` |
| `export` | Writes to `--export-output` file in the specified format |

See [commands reference](references/commands.md) for full details.

## Stop Conditions

Do **not** use this skill when:
- The user needs a **full repo overview** → use **rbuilder-orient** instead
- The user needs **change impact analysis** → use **rbuilder-impact** instead (blast-radius gives richer impact data than raw call chains)
- The user needs **data flow** within a function → use **rbuilder-slice** instead

## Failure Playbook

| Symptom | Fix |
|---------|-----|
| Too many results | Add `LIMIT N` to GQL queries |
| `Symbol not found` | Check spelling with `rbuilder gql --macro-name all_functions unused -f json` |
| Export produces empty file | Check `--query` filter — `name:X` uses exact match; try `all` or `functions` |
| `--query` syntax error | Export uses filter syntax, not GQL MATCH. Use `all`, `name:Foo`, `type:Function`, `functions`, `classes`, or compound with `\|` |

## Example Turn

**User:** "Show me what `PaymentGateway.charge` calls and export a Mermaid diagram."

**Agent:**
1. `rbuilder gql "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE a.name = 'charge' RETURN a,b" -f json | jq '.rows | length'`
2. `rbuilder export --export-format mermaid --export-output charge_calls.mmd --query "name:charge"`

**Reply:** "`PaymentGateway.charge` calls 3 functions. Mermaid diagram exported to `charge_calls.mmd` — you can paste it into any Mermaid renderer."
