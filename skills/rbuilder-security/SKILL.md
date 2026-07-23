---
name: rbuilder-security
description: >-
  Run taint analysis to trace untrusted input from source to sink.
  Use when the user asks about security flows, injection risks,
  taint analysis, data sanitization, or source-to-sink tracking.
  Activates on: taint analysis, security flow, untrusted input,
  source to sink, injection risk, data sanitization, SQL injection,
  XSS, command injection, taint trace.
compatibility: Requires rbuilder CLI (v0.4+) with `discover --with-cfg --with-taint` indexing.
metadata:
  author: rbuilder
  version: "1.0"
---

## Prerequisites

- A `.rbuilder/` directory must exist with CFG and taint data. Run:
  ```bash
  rbuilder discover . --with-cfg --with-taint
  ```
- Both `--with-cfg` and `--with-taint` are required. Without them, taint returns no results.

## Decision Tree

| User intent | Command |
|-------------|---------|
| Taint trace from a variable | `rbuilder slice src/path/File.java --line 42 --variable userInput --taint -f json` |
| Taint with function context | Add `--function processRequest` |
| View taint as CFG | Add `--view cfg` |

> **Note:** There is no `--sink` filter flag. Taint analysis runs against all detected source-to-sink patterns.

## Output Contract

Always use `-f json`. Key fields:

| Field | Type | Meaning |
|-------|------|---------|
| `.taint` | bool | Whether taint was detected |
| `.flows` | number | Total taint flow count |
| `.vulnerable` | number | Number of vulnerable flows |
| `.file` | string | Source file path |
| `.function` | string | Enclosing function |
| `.line` | number | Criterion line |
| `.variable` | string | Criterion variable |

See [commands reference](references/commands.md) for full JSON shapes.

## Stop Conditions

Do **not** use this skill when:
- The question is **not security-related** → use **rbuilder-slice** for general data flow
- The user needs **call-level impact** → use **rbuilder-impact** instead
- The user wants a **repo overview** → use **rbuilder-orient** instead

## Failure Playbook

| Symptom | Fix |
|---------|-----|
| No taint results | Ensure `discover --with-cfg --with-taint` was run |
| `variable not found at line` | Verify the variable name exists at the specified line |
| False positive taint | Review the flow path — rBuilder detects patterns, not semantic intent |
| Three args required error | `slice` always needs: positional file, `--line`, `--variable` |

## Example Turn

**User:** "Is the `userInput` parameter at line 10 of `LoginController.java` vulnerable to SQL injection?"

**Agent:**
1. `rbuilder slice src/auth/LoginController.java --line 10 --variable userInput --function authenticate --taint -f json`
2. Parse `.taint`, `.flows`, `.vulnerable`

**Reply:** "Taint analysis found 2 flows from `userInput` at line 10, with 1 vulnerable path reaching `jdbcTemplate.query()` at line 34 without sanitization. The variable passes through `buildQuery()` at line 22 which concatenates it directly into SQL."
