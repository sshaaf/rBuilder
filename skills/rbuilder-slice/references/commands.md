# rbuilder-slice Command Reference

Detailed flag and JSON output reference for commands used by this skill.
For full TypeScript interface definitions, see [docs/json-api.md](../../docs/json-api.md).

## `slice`

```bash
rbuilder slice FILE --line N --variable VAR -f json
rbuilder slice FILE --line N --variable VAR --direction forward -f json
rbuilder slice FILE --line N --variable VAR --function FUNC --view cfg -f json
```

| Flag | Type | Required | Default | Notes |
|------|------|----------|---------|-------|
| `FILE` (positional) | string | **yes** | — | Source file path (relative to repo root) |
| `--line` | usize | **yes** | — | Source line number |
| `--variable` | string | **yes** | — | Variable name at the criterion |
| `--function` | string | no | — | Enclosing function name (for disambiguation) |
| `--language` | string | no | — | Language hint |
| `--direction` | enum | no | `backward` | `backward` or `forward` |
| `--taint` | bool | no | false | Run taint trace instead of slice (see rbuilder-security) |
| `--view` | enum | no | `text` | `text`, `cfg`, or `pdg` |

### JSON output — `--view text` (`schema_version: 1`)

```json
{
  "schema_version": 1,
  "file": "src/order/handler.java",
  "criterion": { "line": 15, "variable": "request" },
  "direction": "forward",
  "reduction_percent": 73.2,
  "lines": [15, 18, 22, 25, 31, 38, 42, 47],
  "nodes": [
    { "id": 0, "label": "request = ctx.getRequest()", "lines": [15] }
  ],
  "edges": [
    { "from": 0, "to": 1, "kind": "data" }
  ]
}
```

### JSON output — `--view cfg`

```json
{
  "schema_version": 1,
  "file": "src/order/handler.java",
  "function": "handleOrder",
  "view": "cfg",
  "nodes": [
    { "id": 0, "label": "entry", "lines": [14, 15] }
  ],
  "edges": [
    { "from": 0, "to": 1, "label": "true" }
  ]
}
```

### JSON output — `--view pdg`

```json
{
  "schema_version": 1,
  "file": "src/order/handler.java",
  "function": "handleOrder",
  "view": "pdg",
  "nodes": [
    { "id": 0, "label": "request = ctx.getRequest()", "lines": [15] }
  ],
  "edges": [
    { "from": 0, "to": 1, "kind": "data" }
  ]
}
```
