# rbuilder-neighborhood Command Reference

Detailed flag and JSON output reference for commands used by this skill.
For full TypeScript interface definitions, see [docs/json-api.md](../../docs/json-api.md).

## `gql` (call chain queries)

See [rbuilder-orient commands reference](../rbuilder-orient/references/commands.md#gql) for full GQL documentation.

### Common call chain patterns

```bash
# Direct calls from X
rbuilder gql "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE a.name = 'X' RETURN a,b" -f json

# Direct callers of X
rbuilder gql "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE b.name = 'X' RETURN a,b" -f json

# 3-hop transitive callees
rbuilder gql "MATCH (a:Function)-[:CALLS*1..3]->(b:Function) WHERE a.name = 'X' RETURN a,b LIMIT 50" -f json

# All direct calls (built-in macro)
rbuilder gql --macro-name direct_calls unused -f json
```

## `export`

```bash
rbuilder export --export-format mermaid --export-output graph.mmd --query "name:X"
```

| Flag | Type | Notes |
|------|------|-------|
| `--export-format` | enum | **Required.** `json`, `graphml`, `graphviz`, `mermaid` |
| `--export-output` | string | **Required.** Output file path |
| `--query` | string | Filter: `all` (default), `name:Foo`, `type:Function`, `functions`, `classes`, `structs`, `files`, `config`, `name_suffix:X`, `signature:*pattern*`, compound with `\|` |

### Filter syntax (not GQL)

| Filter | Meaning |
|--------|---------|
| `all` | All nodes |
| `name:main` | Exact name match |
| `type:Function` | By node type (case-insensitive) |
| `functions` | Shorthand for `type:Function` |
| `classes` | Shorthand for `type:Class` |
| `name_suffix:Service` | Name ends with |
| `signature:*pattern*` | Signature wildcard match |
| `type:Function\|name_suffix:Service` | Compound intersection (pipe) |
