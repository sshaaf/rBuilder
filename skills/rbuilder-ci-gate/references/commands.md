# rbuilder-ci-gate Command Reference

Detailed flag and JSON output reference for commands used by this skill.
For full TypeScript interface definitions, see [docs/json-api.md](../../docs/json-api.md).
For policy file format, see [docs/policy-format.md](../../docs/policy-format.md).

## `check`

```bash
rbuilder check --policy-file policy.json -f json
rbuilder check --policy-file policy.json
```

| Flag | Type | Required | Notes |
|------|------|----------|-------|
| `--policy-file` | string | **yes** | Path to policy JSON file |

### JSON output (`schema_version: 1`)

```json
{
  "schema_version": 1,
  "policy": "policy.json",
  "violations": [
    {
      "symbol": "OrderProcessor.validate",
      "violation": "impact_zone_size 67 exceeds max 50"
    },
    {
      "symbol": "PaymentGateway.charge",
      "error": "impact_zone_size 53 exceeds max 50"
    }
  ],
  "passed": false
}
```

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | All checks pass — no violations |
| `1` | Policy violations found (or command error) |

### CI integration

```yaml
# GitHub Actions example
- name: rBuilder policy check
  run: |
    rbuilder discover .
    rbuilder check --policy-file policy.json
```

The exit code `1` on violations automatically fails the CI step.
