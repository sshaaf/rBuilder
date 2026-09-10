# CI Policy Checks

## Introduction

rgctl provides two policy gateways:

| Command | Graphs | Git scope | Temporal (new / existing / resolved) |
|---------|--------|-----------|--------------------------------------|
| **`check`** | One (`.rgctl/` after `discover`) | Working tree vs `HEAD` by default; optional `--base-ref` / `--head-ref` | No |
| **`pr-check`** | Two (base + head snapshots) | `git diff base-ref head-ref` (commits) | Yes |

Both evaluate blast-radius rules from a JSON policy file and exit **1** on failure. Use **`check`** for local pre-commit and single-snapshot CI. Use **`pr-check`** for pull requests where you only want to block **new** violations vs `main`.

Policy checks automate what would otherwise be manual code review: impact zone limits, centrality alerts, and domain isolation boundaries.

## Use Cases

- **CI pipeline integration.** Add `check` as a build step to catch architectural violations before merge.
- **Blast-radius guardrails.** Prevent changes to functions whose impact zone exceeds a threshold.
- **Centrality alerts.** Flag functions with high centrality scores that may need extra review.
- **Domain isolation.** Enforce separation between modules that should not depend on each other.
- **Continuous architecture monitoring.** Track policy compliance over time as the codebase evolves.

## Example Project

This guide uses the **CoolStore** (`example/coolstore`). Make sure you have run `discover` first:

```bash
rgctl -r example/coolstore discover
```

## Step-by-Step

### 1. Examine the Policy File

The CoolStore example ships with a policy file at `example/coolstore/policy.json`:

```json
{"max_impact_nodes": 15, "centrality_alert_threshold": 0.8}
```

This policy defines two rules:

| Rule | Value | Meaning |
|------|-------|---------|
| `max_impact_nodes` | 15 | A function's blast-radius impact zone must not exceed 15 nodes |
| `centrality_alert_threshold` | 0.8 | Functions with centrality above 0.8 trigger an alert |

### 2. Run the Policy Check

```bash
rgctl -r example/coolstore -f json check \
  --policy-file example/coolstore/policy.json
```

**Output (truncated):**

```json
{
  "passed": false,
  "policy": "example/coolstore/policy.json",
  "schema_version": 1,
  "violations": [
    {
      "error": "Graph error: scale failure: impact zone size 114 exceeds max 15",
      "symbol": "baseIsEqual"
    },
    {
      "error": "Graph error: scale failure: impact zone size 648 exceeds max 15",
      "symbol": "indexOf"
    },
    {
      "error": "Graph error: scale failure: impact zone size 110 exceeds max 15",
      "symbol": "getMatchData"
    },
    {
      "error": "Graph error: scale failure: impact zone size 16 exceeds max 15",
      "symbol": "_fnInitComplete"
    },
    {
      "error": "Graph error: scale failure: impact zone size 456 exceeds max 15",
      "symbol": "trimmedLeftIndex"
    }
  ]
}
```

**What this tells you:**

- **`passed: false`** -- the codebase has policy violations.
- **`violations`** -- each violation lists the offending symbol and the rule it broke.
- `indexOf` has the largest impact zone at 648 nodes -- changing this function could affect 648 other functions.
- `baseIsEqual` (impact: 114) and `trimmedLeftIndex` (impact: 456) are lodash utility functions deeply embedded in the call graph.
- `_fnInitComplete` barely exceeds the threshold at 16 nodes.

### 3. Check the Exit Code

The `check` command exits with code 1 on violations, making it suitable for CI:

```bash
rgctl -r example/coolstore check \
  --policy-file example/coolstore/policy.json
echo "Exit code: $?"
```

```
Exit code: 1
```

In a CI pipeline:

```yaml
# GitHub Actions example
- name: Architecture check
  run: rgctl -r . check --policy-file policy.json
```

If any violation is found, the step fails and the build is blocked.

### 4. Text Format for Human Review

Use text format for readable output in pull request comments:

```bash
rgctl -r example/coolstore check \
  --policy-file example/coolstore/policy.json
```

### 5. Per-Function Policy Check with Blast Radius

You can also apply a policy to a single function using `blast-radius --policy-file`:

```bash
rgctl -r example/coolstore -f json blast-radius priceShoppingCart \
  --policy-file example/coolstore/policy.json
```

This runs blast-radius analysis on `priceShoppingCart` and checks the result against the policy. The `gatekeeping` section of the output shows whether the function passes or violates the policy.

### 6. Writing a Custom Policy

Create a policy file tailored to your project:

```json
{
  "max_impact_nodes": 25,
  "centrality_alert_threshold": 0.7
}
```

Stricter policies (lower thresholds) catch more violations; permissive policies (higher thresholds) only flag extreme cases.

The `docs/examples/` directory contains example policies:

| File | Purpose |
|------|---------|
| `policy-strict.json` | Tight thresholds for well-modularized codebases |
| `policy-permissive.json` | Relaxed thresholds for monoliths in early migration |

## Policy File Format

```json
{
  "max_impact_nodes": <integer>,
  "centrality_alert_threshold": <float 0.0-1.0>
}
```

| Field | Type | Description |
|-------|------|-------------|
| `max_impact_nodes` | integer | Maximum allowed impact zone size for any function |
| `centrality_alert_threshold` | float | Centrality score above which a function triggers a violation |

See the [Policy Format Reference](../policy-format.md) for the full schema.

## Understanding Violations

| Violation Type | Message Pattern | Cause |
|----------------|----------------|-------|
| Scale failure | `impact zone size N exceeds max M` | A function's blast radius exceeds `max_impact_nodes` |
| Centrality alert | `centrality N exceeds threshold M` | A function's centrality score exceeds `centrality_alert_threshold` |
| Domain isolation | `cross-domain call from A to B` | A function calls across a domain boundary |
| Cascade hazard | `cascade depth N exceeds max M` | A function's call chain depth exceeds the maximum |

## Benefits

- **Automated architecture enforcement.** Replace manual review with machine-checked policies.
- **Clear exit codes.** Exit 0 = pass, exit 1 = violations -- integrates with any CI system.
- **Structured output.** JSON violations are easy to parse, aggregate, and trend over time.
- **Customizable thresholds.** Tune policies to match your project's maturity and architecture goals.
- **Preventive, not reactive.** Catch architectural drift before it ships, not after it causes problems.

---

## Deterministic node IDs (migration)

As of the **temporal-delta** release line, graph node UUIDs are derived deterministically from `(file_path, name, node_type)` when a file path is known. This keeps cross-file call edges stable when only one file is re-indexed.

**One-time upgrade** after pulling this change:

```bash
rm -rf .rgctl .rgctl-base
rgctl discover .
```

Symbols without a file path (e.g. orphan env-var nodes) still receive random UUIDs. External stubs under `<external>` use deterministic IDs from their qualified name + stub path.

---

## `pr-check`: temporal PR gate

Compares a **base** graph (usually `main`) with a **head** graph (PR branch), scopes policy to files changed in git, and classifies violations:

| Class | Meaning | Fails when `new_violations_only: true` |
|-------|---------|----------------------------------------|
| `new` | Violation on head, not on base | **Yes** |
| `existing` | Violation on both | No |
| `resolved` | Violation on base, fixed on head | No |

Example policy: [rgctl-tests/rgctl-pr-policy.json](../../rgctl-tests/rgctl-pr-policy.json).

### Artifact layout

Both artifacts use the same layout as a normal `discover` output:

```text
{artifact-root}/
  .rgctl/
    graph.snapshot.bin    ← required
    analysis_results.bin  ← optional; hydrated for policy
```

### Optional parameters (defaults)

| Flag | Default | Resolution |
|------|---------|------------|
| `--head-artifact` | **`-r` repo / cwd** | `{repo}/.rgctl/graph.snapshot.bin` |
| `--base-artifact` | see below | explicit flag → `$RGCTL_BASE_ARTIFACT` → `{repo}/.rgctl-base/` |
| `--base-ref` | `origin/main` | Left side of `git diff` |
| `--head-ref` | `HEAD` | Right side of `git diff` |
| `--strict` | off | Fail when git reports zero changed files |
| `--cascade-depth` | `1` | Reverse call-dependency hops when synthesizing delta head (`0` = off) |
| `--full-snapshots` | off | Require pre-built head snapshot (legacy dual-discover CI) |

**Delta head (default):** when `--head-artifact` is omitted and `--full-snapshots` is not set,
`pr-check` copies the base snapshot into `{repo}/.rgctl/`, applies the git name-status delta via
`GraphCompactor`, and evaluates policy — no second full `discover` on the PR branch.

### CI cache layout (`.rgctl-cache/{sha}/`)

Store one immutable base artifact per merge-base commit; PR jobs only run delta synthesis:

```text
.rgctl-cache/
  abc1234/                 # merge-base or main SHA
    .rgctl/
      graph.snapshot.bin
      file_hashes.json
  def5678/
    .rgctl/
      ...
```

```yaml
# GitHub Actions (sketch)
- name: Restore base graph
  uses: actions/cache@v4
  with:
    path: .rgctl-cache/${{ github.event.pull_request.base.sha }}
    key: rgctl-base-${{ github.event.pull_request.base.sha }}

- name: PR policy gate
  run: |
    export RGCTL_BASE_ARTIFACT=".rgctl-cache/${{ github.event.pull_request.base.sha }}"
    rgctl -r . -f json pr-check \
      --policy-file rgctl-tests/rgctl-pr-policy.json \
      --base-ref origin/${{ github.base_ref }} \
      --head-ref HEAD \
      --strict
```

**Minimal command** (after preparing `.rgctl-base/` — see scenario 4):

```bash
rgctl -r "$REPO" -f json pr-check \
  --policy-file rgctl-tests/rgctl-pr-policy.json \
  --base-ref origin/main \
  --head-ref HEAD \
  --strict
```

**Prepare local base cache** (one-time per `main` refresh):

```bash
git checkout main && rgctl discover .
mkdir -p .rgctl-base && cp -a .rgctl .rgctl-base/
git checkout -
rgctl discover .   # head branch
```

### JSON output

```json
{
  "schema_version": "2",
  "passed": true,
  "violations": [],
  "violations_summary": { "new": 0, "existing": 0, "resolved": 0, "regression": 0 },
  "graph_diff": {
    "nodes_added": 0,
    "nodes_removed": 0,
    "nodes_changed": 0,
    "edges_added": 0,
    "edges_removed": 0
  },
  "scope": { "files": 12, "entities": 4 }
}
```

Save reports for user testing:

```bash
rgctl -r . -f json pr-check --policy-file rgctl-tests/rgctl-pr-policy.json \
  > reports/pr-check-$(git rev-parse --short HEAD).json
```

---

## User testing scenarios

Use these flows when validating policy gates in a real repo. Each lists **command**, **params**, and **files read/written**.

### Scenario 1 — Temporal change between two git versions

**Goal:** Compare policy impact between tag/commit A and B (e.g. `v1.0` → `v2.0`).

**Command:** `pr-check` (two `discover` runs required).

```bash
# Index version A
git checkout v1.0
rgctl -r . discover .
cp -a .rgctl /tmp/snapshots/v1.0-rgctl

# Index version B
git checkout v2.0
rgctl -r . discover .

rgctl -r . -f json pr-check \
  --policy-file rgctl-tests/rgctl-pr-policy.json \
  --base-artifact /tmp/snapshots/v1.0-rgctl \
  --head-artifact . \
  --base-ref v1.0 \
  --head-ref v2.0 \
  --strict \
  > reports/temporal-v1-v2.json
```

| Read | Written |
|------|---------|
| `/tmp/snapshots/v1.0-rgctl/.rgctl/graph.snapshot.bin` | `reports/temporal-v1-v2.json` |
| `.rgctl/graph.snapshot.bin` | exit code |

---

### Scenario 2 — Dirty working tree vs last commit

**Goal:** Check uncommitted edits against the graph without a base snapshot.

**Command:** **`check`** (not `pr-check` — `pr-check` only diffs **commits**).

```bash
rgctl -r . discover .    # refresh graph if code changed materially
rgctl -r . -f json check \
  --policy-file rgctl-tests/rgctl-pr-policy.json \
  --strict \
  > reports/dirty-tree-check.json
```

Omit `--base-ref` / `--head-ref` so scope is `git diff --name-only HEAD` (working tree vs last commit).

| Read | Written |
|------|---------|
| `.rgctl/graph.snapshot.bin` | `reports/dirty-tree-check.json` |
| `policy.json`, dirty files via git | exit code |

---

### Scenario 3 — Check changes and document

**Goal:** Run policy on a committed branch delta and archive JSON for review.

**Option A — single graph (`check`):**

```bash
rgctl -r . -f json check \
  --policy-file rgctl-tests/rgctl-pr-policy.json \
  --base-ref origin/main \
  --head-ref HEAD \
  --strict \
  > reports/check-$(git rev-parse --short HEAD).json
```

**Option B — full PR report with graph diff + temporal classes (`pr-check`):**

```bash
rgctl -r . -f json pr-check \
  --policy-file rgctl-tests/rgctl-pr-policy.json \
  --base-ref origin/main \
  --head-ref HEAD \
  --strict \
  > reports/pr-check-$(git rev-parse --short HEAD).json
```

Requires base artifact (`.rgctl-base/` or `$RGCTL_BASE_ARTIFACT` or `--base-artifact`).

---

### Scenario 4 — PR check against `main` (CI)

**Goal:** Block merges that introduce **new** policy violations vs `main`.

**CI on `main`:** `discover` → upload `.rgctl/` as artifact.

**CI on PR:**

```bash
rgctl discover .
rgctl -r . -f json pr-check \
  --policy-file rgctl-tests/rgctl-pr-policy.json \
  --base-artifact "$RGCTL_BASE_ARTIFACT" \
  --base-ref origin/main \
  --head-ref HEAD \
  --strict
```

With defaults only (local `.rgctl-base/` prepared):

```bash
export RGCTL_BASE_ARTIFACT=/tmp/rgctl-main-cache   # optional; or use .rgctl-base/
rgctl -r . -f json pr-check \
  --policy-file rgctl-tests/rgctl-pr-policy.json \
  --strict
```

| Read | Written |
|------|---------|
| Main cache `.rgctl/graph.snapshot.bin` | stdout JSON |
| PR `.rgctl/graph.snapshot.bin` | exit 1 if any `new` violation |
| `git diff origin/main HEAD` | |

---

### Scenario 5 — Scoped `check` without temporal comparison

**Goal:** Fast gate on a feature branch using one snapshot (no base cache).

```bash
rgctl -r . discover .
rgctl -r . -f json check \
  --policy-file policy.json \
  --base-ref HEAD~10 \
  --head-ref HEAD \
  --strict
```

Use when you do not have a `main` artifact yet but want commit-range scoping on the current graph.

---

## `check` diff scoping flags

| Flag | Effect |
|------|--------|
| *(none)* | `git diff --name-only HEAD` — working tree vs last commit |
| `--base-ref` + `--head-ref` | `git diff --name-only base head` between commits |
| `--strict` | Fail if diff scope is empty or no functions match (no fallback to all functions) |

Policy field `scope.strict_diff: true` also enables strict mode for `check`.

---

## Related Guides

- [Discovering and Indexing a Codebase](discovering-and-indexing.md) -- must run `discover` before `check` / `pr-check`
- [Blast Radius Analysis](blast-radius-analysis.md) -- the per-function analysis that policy commands run at scale
- [Graph Metrics](graph-metrics.md) -- centrality scores that feed into policy checks
- [Migration Planning](migration-planning.md) -- combine policy checks with migration roadmaps
- [Graph diff design](../design/graph-diff-design.md) -- snapshot diff internals for `pr-check`
