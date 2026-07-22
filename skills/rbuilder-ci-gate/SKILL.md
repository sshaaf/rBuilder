---
name: rbuilder-ci-gate
description: >-
  Enforce blast radius policies in CI pipelines.
  Use when the user asks about CI policy checks, merge gates,
  blast radius limits, quality gates, or policy enforcement.
  Activates on: CI check, policy enforcement, blast radius limit,
  merge gate, quality gate, policy file, CI pipeline, build gate,
  policy violation.
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
- A policy file must exist (JSON format). See [docs/policy-format.md](../../docs/policy-format.md).

## Decision Tree

| User intent | Command |
|-------------|---------|
| Check against policy | `rbuilder check --policy-file policy.json -f json` |
| CI pipeline gate (exit code) | `rbuilder check --policy-file policy.json` (exit 0 = pass, exit 1 = violations) |

> **Note:** There is no `-s` flag — `check` evaluates all git-changed functions against the policy.

## Output Contract

Always use `-f json`. Key fields:

| Field | Type | Meaning |
|-------|------|---------|
| `.passed` | bool | `true` if all checks pass |
| `.policy` | string | Path to policy file used |
| `.violations[]` | array | List of violations (empty when passed) |
| `.violations[].symbol` | string | Function that violated the policy |
| `.violations[].error` | string | Error details (if applicable) |
| `.violations[].violation` | string | Which policy rule was violated |

See [commands reference](references/commands.md) for full JSON shapes.

## Stop Conditions

Do **not** use this skill when:
- **No policy file exists** — help the user create one first (see [docs/policy-format.md](../../docs/policy-format.md))
- The user wants **exploratory analysis**, not CI enforcement → use **rbuilder-impact** instead
- The user wants to **understand** impact before writing policies → use **rbuilder-impact** first

## Failure Playbook

| Symptom | Fix |
|---------|-----|
| `policy file not found` | Verify path to policy file |
| All checks fail | Verify policy file is valid **JSON** (not YAML) |
| Exit code 1 | Expected when violations are found — not an error |
| `graph snapshot not found` | Run `rbuilder discover .` first |

## Example Turn

**User:** "Set up a CI gate that fails if any function has a blast radius over 50."

**Agent:**
1. Create `policy.json`:
   ```json
   {
     "rules": [
       { "metric": "impact_zone_size", "max": 50 }
     ]
   }
   ```
2. `rbuilder check --policy-file policy.json -f json`
3. Parse `.passed` and `.violations[]`

**Reply:** "Policy check found 2 violations: `OrderProcessor.validate` has an impact zone of 67 (limit 50) and `PaymentGateway.charge` has 53. These functions exceed the blast radius threshold. Exit code 1 blocks the CI pipeline."
