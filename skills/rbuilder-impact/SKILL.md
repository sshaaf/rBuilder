---
name: rbuilder-impact
description: >-
  Analyze change impact before refactoring, renaming, or deleting symbols.
  Use when the user asks about blast radius, who calls a function, whether
  a change is safe, or what breaks if something is modified.
  Activates on: refactor, rename, delete symbol, change impact, who calls this,
  blast radius, safe to change, what depends on this, callers, impact analysis.
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
| Impact of changing symbol X | `rbuilder blast-radius X -f json` |
| Disambiguate symbol by class | `rbuilder blast-radius X --class MyClass -f json` |
| Disambiguate symbol by file | `rbuilder blast-radius X --file src/path/File.java -f json` |
| Limit impact depth to N hops | `rbuilder blast-radius X --depth N -f json` |
| Who calls X (direct callers) | `rbuilder gql "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE b.name = 'X' RETURN a" -f json` |
| Full caller chain (up to 3 hops) | `rbuilder gql "MATCH (a:Function)-[:CALLS*1..3]->(b:Function) WHERE b.name = 'X' RETURN a,b" -f json` |

## Output Contract

Always use `-f json`. Key fields to surface:

| Command | Key fields |
|---------|-----------|
| `blast-radius` | `.target` (symbol info), `.metrics.score`, `.metrics.direct_callers_count`, `.metrics.impact_zone_size`, `.topology.direct_callers[]`, `.topology.impact_zone[]`, `.gatekeeping.policy_status` |
| `gql` | `.rows[]` — each row is an array of bindings with `.binding`, `.node`, `.type`, `.file` |

See [commands reference](references/commands.md) for full JSON shapes.

## Stop Conditions

Do **not** use this skill when:
- The user wants a **repo overview** → use **rbuilder-orient** instead
- The user wants to trace **data flow within a function** → use **rbuilder-slice** instead
- The user wants **taint/security analysis** → use **rbuilder-security** instead
- The user wants to **export a subgraph** → use **rbuilder-neighborhood** instead

## Failure Playbook

| Symptom | Fix |
|---------|-----|
| `Multiple symbols match` | Add `--class ClassName` or `--file path/to/File.java` |
| `Symbol not found` | Check spelling; use `rbuilder gql --macro-name all_functions unused -f json` to list available symbols |
| Stale results after code changes | Re-run `rbuilder discover .` to rebuild the graph |
| `.gatekeeping.violations` is non-empty | Policy violations found — review `.gatekeeping.violations[].violation` |

## Example Turn

**User:** "Is it safe to rename `OrderProcessor.validate`?"

**Agent:**
1. `rbuilder blast-radius OrderProcessor.validate -f json`
2. Parse `.metrics.direct_callers_count` and `.metrics.impact_zone_size`
3. Review `.topology.direct_callers[]` for affected symbols

**Reply:** "Renaming `OrderProcessor.validate` affects 4 direct callers and 12 functions in the transitive impact zone. The direct callers are `CheckoutService.process`, `BatchRunner.run`, `ApiController.submit`, and `TestHelper.setup`. I'd recommend updating all 4 call sites in one PR."
