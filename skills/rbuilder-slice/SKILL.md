---
name: rbuilder-slice
description: >-
  Trace data flow at the line level using program slicing.
  Use when the user asks where a variable flows, what affects a line,
  backward or forward slicing, or line-level data dependency analysis.
  Activates on: data flow, variable tracking, where does this value go,
  backward slice, forward slice, line-level analysis, what affects this line,
  program slice, data dependency.
compatibility: Requires rbuilder CLI (v0.4+) with `discover --with-cfg` indexing.
metadata:
  author: rbuilder
  version: "1.0"
---

## Prerequisites

- A `.rbuilder/` directory must exist with CFG data. Run:
  ```bash
  rbuilder discover . --with-cfg
  ```
- The `--with-cfg` flag is required for slice to work. Without it, slice returns empty results.

## Decision Tree

| User intent | Command |
|-------------|---------|
| Where does variable flow (forward) | `rbuilder slice src/path/File.java --line 42 --variable request --direction forward -f json` |
| What affects this variable (backward) | `rbuilder slice src/path/File.java --line 42 --variable request -f json` |
| Disambiguate by function name | Add `--function handleRequest` |
| View as control flow graph | Add `--view cfg` |
| View as program dependence graph | Add `--view pdg` |

> **Three arguments are always required:** positional file path, `--line`, and `--variable`. The `--direction` defaults to `backward`.

## Output Contract

Always use `-f json`. Response shape depends on `--view`:

| View | Key fields |
|------|-----------|
| `text` (default) | `.criterion` (line, variable), `.direction`, `.reduction_percent`, `.lines[]`, `.nodes[]`, `.edges[]` |
| `cfg` | `.nodes[]` (CfgBlockNode — id, label, lines), `.edges[]` (CfgEdge — from, to, label) |
| `pdg` | `.nodes[]` (PdgGraphNode — id, label, lines), `.edges[]` (PdgGraphEdge — from, to, kind) |

See [commands reference](references/commands.md) for full JSON shapes.

## Stop Conditions

Do **not** use this skill when:
- The user asks about **callers** of a function → use **rbuilder-impact** instead (blast-radius for impact, gql for call chains)
- The user needs **taint/security analysis** → use **rbuilder-security** instead
- The user wants a **call chain** → use **rbuilder-neighborhood** instead

## Failure Playbook

| Symptom | Fix |
|---------|-----|
| Empty slice result | Ensure `discover --with-cfg` was run (CFG data required) |
| `variable not found at line` | Verify the variable name exists at the specified line in the source file |
| Wrong function matched | Add `--function functionName` to disambiguate |
| `file not found` | Use the path relative to the repo root |

## Example Turn

**User:** "Where does the `request` variable flow in `handleOrder` at line 15?"

**Agent:**
1. `rbuilder slice src/order/handler.java --line 15 --variable request --function handleOrder --direction forward -f json`
2. Parse `.lines[]` for affected line numbers and `.reduction_percent` for scope

**Reply:** "The `request` variable at line 15 flows forward to 8 statements (lines 15, 18, 22, 25, 31, 38, 42, 47) — a 73% slice reduction. It reaches the database call at line 42 via `repository.save(request.getOrder())`."
