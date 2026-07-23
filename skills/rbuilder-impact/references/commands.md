# rbuilder-impact Command Reference

Detailed flag and JSON output reference for commands used by this skill.
For full TypeScript interface definitions, see [docs/json-api.md](../../docs/json-api.md).

## `blast-radius`

```bash
rbuilder blast-radius SYMBOL -f json
rbuilder blast-radius SYMBOL --class ClassName -f json
rbuilder blast-radius SYMBOL --depth 3 -f json
```

| Flag | Type | Notes |
|------|------|-------|
| `SYMBOL` (positional) | string | **Required.** Function name, UUID, or FQN (`Class::method`) |
| `--depth` | usize | Limit upstream impact to N call hops (default: full transitive closure) |
| `--class` | string | Class/namespace filter for disambiguation |
| `--file` | string | Source file path filter for disambiguation |
| `--with-slices` | bool | Run statement-level slice hand-off analysis |
| `--policy-file` | string | Path to policy JSON file |
| `--no-policy` | bool | Skip policy evaluation |

### JSON output (`schema_version: 2`)

```json
{
  "schema_version": 2,
  "target": {
    "name": "validate",
    "fqn": "OrderProcessor.validate",
    "file": "src/order/processor.java",
    "language": "java",
    "canonical_fqn": "com.example.order.OrderProcessor.validate"
  },
  "metrics": {
    "score": 0.73,
    "direct_callers_count": 4,
    "impact_zone_size": 12,
    "caller_depth_limit": null
  },
  "topology": {
    "scc_component_id": null,
    "direct_callers": [
      { "name": "CheckoutService.process", "file": "src/checkout/service.java" }
    ],
    "impact_zone": [
      { "name": "ApiController.submit", "file": "src/api/controller.java" }
    ]
  },
  "gatekeeping": {
    "policy_status": "no_policy",
    "violations": [],
    "handoffs": []
  }
}
```

## `gql` (caller queries)

```bash
rbuilder gql "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE b.name = 'X' RETURN a" -f json
```

See [rbuilder-orient commands reference](../rbuilder-orient/references/commands.md#gql) for full GQL flag and output documentation.
