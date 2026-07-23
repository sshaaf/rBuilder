# rbuilder-orient Command Reference

Detailed flag and JSON output reference for commands used by this skill.
For full TypeScript interface definitions, see [docs/json-api.md](../../docs/json-api.md).

## `discover`

```bash
rbuilder discover [PATH] -f json
```

| Flag | Type | Default | Notes |
|------|------|---------|-------|
| `PATH` (positional) | string | cwd | Repository path |
| `--languages` / `-l` | string | all | Language filter |
| `--exclude` / `-e` | string | none | Exclude patterns |
| `-f json` | global | text | Output format |

### JSON output (`schema_version: 2`)

```json
{
  "schema_version": 2,
  "command": "discover",
  "metrics": {
    "files_discovered": 342,
    "files_indexed": 310,
    "files_skipped": 32,
    "nodes_generated": 1847,
    "edges_generated": 5203,
    "duration_ms": 1523
  }
}
```

## `metrics`

```bash
rbuilder metrics [--pagerank] [--betweenness] [--communities] -f json
```

When no section flag is passed, all three sections are included.

| Flag | Type | Notes |
|------|------|-------|
| `--pagerank` | bool | Include PageRank section |
| `--betweenness` | bool | Include betweenness section |
| `--communities` | bool | Include communities section |
| `--iterations` | usize | PageRank iteration count |

### JSON output (`schema_version: 1`)

```json
{
  "schema_version": 1,
  "pagerank": {
    "top": [
      { "node": "<uuid>", "pagerank": 0.034 }
    ],
    "converged": true,
    "iterations": 100,
    "max_delta": 0.0001
  },
  "betweenness": [
    { "node": "<uuid>", "score": 0.15 }
  ],
  "communities": {
    "count": 12,
    "modularity": 0.45,
    "assignments": 1847
  }
}
```

## `gql`

```bash
rbuilder gql "MATCH (n:Function) RETURN n LIMIT 20" -f json
rbuilder gql --macro-name all_functions unused -f json
```

| Flag | Type | Notes |
|------|------|-------|
| `query` (positional) | string | **Required.** GQL MATCH query (or `unused` with `--macro-name`) |
| `--macro-name` | string | Use built-in macro: `all_functions`, `direct_calls`, `call_chain` |
| `--explain` | bool | Show query plan |

### GQL syntax

```
MATCH (n:Function) RETURN n
MATCH (n:Function) WHERE n.name LIKE '*Service*' RETURN n LIMIT 20
MATCH (n:Function) WHERE n.name = 'main' RETURN n
MATCH (a:Function)-[:CALLS*1..3]->(b:Function) RETURN a,b LIMIT 50
```

Operators: `=` (exact), `LIKE` (glob with `*`), `AND` for combining predicates.

### JSON output (`schema_version: 1`)

```json
{
  "schema_version": 1,
  "rows": [
    [
      { "binding": "n", "node": "ShoppingCartService.checkout", "type": "Function", "file": "src/cart/service.java" }
    ]
  ],
  "count": 1,
  "explain": false
}
```

## `semantic query`

```bash
rbuilder semantic query "authentication logic" -f json --limit 10
```

| Flag | Type | Default | Notes |
|------|------|---------|-------|
| `TEXT` (positional) | string | — | Natural-language or keyword query |
| `--limit` | usize | 20 | Max hits |
| `--expand` | enum | none | `neighbors`, `blast`, `gql`, `all` |
| `--no-fusion` | bool | false | Disable late fusion re-ranking |

Requires `rbuilder semantic index` to have been run first.
