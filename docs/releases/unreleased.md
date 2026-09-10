# Unreleased (post v0.4.12)

## Graph / CI policy

- **Deterministic node IDs:** `Node.id` is now derived from `(file_path, name, node_type)` via `StableNodeKey` + UUID v5 when a file path is set. Cross-file call edges survive single-file re-extract + compaction. **Migration:** delete `.rgctl` / `.rgctl-base` and run `rgctl discover .` once (see [CI Policy Checks](../guides/ci-policy-checks.md#one-time-setup)).
- **Delta `pr-check` head:** By default, `pr-check` synthesizes the head snapshot from the cached base artifact + git name-status delta (no second full `discover`). Use `--full-snapshots` for the legacy dual-artifact path. `discover --files PATH,...` runs incremental updates on selected paths.
- **Violation timeline:** `pr-check` JSON schema v2 adds `violations_summary`, `regression` class (ledger-backed), `--bisect` (`introduced_in_commit`), `--synthetic-head worktree`, and append-only `.rgctl/violation_ledger.jsonl`. `check --temporal` delegates to the same evaluator.
- **Calendar policies:** optional `temporal` block (`effective_from`, `grace_period_days`, `violation_sla_days`, `sunset_date`, …) with `--strict-calendar`; SLA uses ledger `first_seen`. Example workflow: `.github/workflows/rgctl-pr-check.yml`.
