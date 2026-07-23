# rbuilder-security Command Reference

Detailed flag and JSON output reference for commands used by this skill.
For full TypeScript interface definitions, see [docs/json-api.md](../../docs/json-api.md).

## `slice --taint`

```bash
rbuilder slice FILE --line N --variable VAR --taint -f json
rbuilder slice FILE --line N --variable VAR --function FUNC --taint -f json
rbuilder slice FILE --line N --variable VAR --taint --view cfg -f json
```

All flags from `slice` apply (see [rbuilder-slice commands reference](../rbuilder-slice/references/commands.md)). The `--taint` flag switches from program slicing to taint analysis mode.

### JSON output — taint mode (`schema_version: 1`)

```json
{
  "schema_version": 1,
  "file": "src/auth/LoginController.java",
  "function": "authenticate",
  "line": 10,
  "variable": "userInput",
  "taint": true,
  "flows": 2,
  "vulnerable": 1
}
```

### Indexing requirements

Taint analysis requires two discover flags:
- `--with-cfg` — builds per-function control flow graphs
- `--with-taint` — runs discover-time taint pattern detection

```bash
rbuilder discover . --with-cfg --with-taint
```

Without `--with-taint`, the `--taint` flag on `slice` returns zero flows.
