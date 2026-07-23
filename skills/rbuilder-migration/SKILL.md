---
name: rbuilder-migration
description: >-
  Generate and inspect migration plans for large-scale refactors.
  Use when the user asks about migration strategy, batch refactoring,
  dependency-aware ordering, or upgrading a codebase systematically.
  Activates on: migration plan, upgrade dependencies, batch refactor,
  large-scale changes, migration strategy, modernization, rewrite plan,
  migration order, dependency ordering.
compatibility: Requires rbuilder CLI (v0.4+). Run `rbuilder --version` to verify.
metadata:
  author: rbuilder
  version: "1.0"
---

## Prerequisites

- Run a full discover with all analysis flags:
  ```bash
  rbuilder discover . --with-cfg --with-taint --with-security --with-dashboard --with-harmonic --export-migration-hints
  ```
- There is **no `--all` shorthand** — each `--with-*` flag must be passed individually.

## Decision Tree

| User intent | Command |
|-------------|---------|
| Generate migration plan | `rbuilder discover . --with-cfg --with-taint --with-security --with-dashboard --with-harmonic --export-migration-hints` |
| Use foundational-first preset | Add `--migration-preset foundational_first` |
| Use dense cluster preset | Add `--migration-preset dense_cluster` |
| Use risk mitigation preset | Add `--migration-preset risk_mitigation` |
| Priority ordering (score-only) | Add `--migration-order priority` |
| Scheduled ordering (topological) | `--migration-order scheduled` (default) |
| Inspect the plan | Read `.rbuilder/migration_plan.json` |
| View in dashboard | `rbuilder serve --open` → Migration tab |

### Migration presets

| Preset | Strategy |
|--------|----------|
| `hybrid_default` | Balanced α·PageRank + β·Harmonic − γ·Blast |
| `foundational_first` | Prioritize high-PageRank foundational packages |
| `dense_cluster` | Extract dense community clusters first |
| `risk_mitigation` | Minimize blast radius risk |

## Output Contract

The migration plan is written to `.rbuilder/migration_plan.json`. Key fields:

| Field | Meaning |
|-------|---------|
| `packages[]` | Ordered list of packages to migrate |
| `packages[].name` | Package name |
| `packages[].priority_score` | Computed priority (higher = migrate first) |
| `packages[].step` | Scheduled step number (topological order) |
| `packages[].functions` | Functions in the package |

See [commands reference](references/commands.md) for full JSON shapes.

## Stop Conditions

Do **not** use this skill when:
- The user wants to change a **single symbol** → use **rbuilder-impact** instead
- No actual migration is planned — use other skills for exploratory analysis
- The user wants **CI enforcement** → use **rbuilder-ci-gate** instead

## Failure Playbook

| Symptom | Fix |
|---------|-----|
| Empty migration plan | Ensure all `--with-*` flags are passed (especially `--with-harmonic`) |
| Missing packages | Re-run `discover` with all flags — partial indexing produces partial plans |
| Plan has wrong ordering | Try a different `--migration-preset` or `--migration-order` |

## Example Turn

**User:** "Create a migration plan prioritizing foundational packages."

**Agent:**
1. `rbuilder discover . --with-cfg --with-taint --with-security --with-dashboard --with-harmonic --export-migration-hints --migration-preset foundational_first`
2. Read `.rbuilder/migration_plan.json` and parse `packages[:5]`

**Reply:** "Migration plan generated with foundational-first ordering. Top 5 packages to migrate: `core.utils` (score 0.89), `data.models` (0.82), `auth.service` (0.74), `api.gateway` (0.68), `order.processing` (0.61). The plan has 12 packages across 8 scheduled steps."
