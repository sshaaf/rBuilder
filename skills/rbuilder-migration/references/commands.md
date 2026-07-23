# rbuilder-migration Command Reference

Detailed flag and JSON output reference for commands used by this skill.
For full documentation, see [docs/json-api.md](../../docs/json-api.md) §12 and [building-migration-plan.md](../../docs/building-migration-plan.md).

## `discover` (migration mode)

```bash
rbuilder discover . \
  --with-cfg --with-taint --with-security \
  --with-dashboard --with-harmonic \
  --export-migration-hints \
  --migration-preset foundational_first \
  --migration-order priority
```

### Migration-specific flags

| Flag | Type | Default | Notes |
|------|------|---------|-------|
| `--export-migration-hints` | bool | false | Write migration plan JSON. Alias: `--export-migration-plan` (deprecated) |
| `--migration-preset` | enum | `hybrid_default` | `hybrid_default`, `foundational_first`, `dense_cluster`, `risk_mitigation` |
| `--migration-order` | enum | `scheduled` | `scheduled` (topological) or `priority` (score-only) |
| `--with-harmonic` | bool | false | Compute harmonic centrality (used in priority scoring) |

### All `--with-*` flags (required for full migration)

| Flag | Purpose |
|------|---------|
| `--with-cfg` | Per-function CFG, dominators, PDG |
| `--with-taint` | Discover-time taint analysis |
| `--with-security` | Secret scanning |
| `--with-dashboard` | Static dashboard bundle export |
| `--with-harmonic` | Harmonic centrality for migration scoring |

### Output file

Default path: `.rbuilder/migration_plan.json`

Override with `-o /custom/path.json`.

The dashboard copy is at `.rbuilder/dashboard/migration_plan.json` (when `--with-dashboard` is set).
