# rBuilder for AI agents

rBuilder is designed so agents answer **structural questions** from a pre-built graph instead of reading whole files into context.

**Full JSON reference:** [docs/json-api.md](docs/json-api.md)  
**Copy-paste recipes:** [docs/agent-recipes.md](docs/agent-recipes.md)

---

## Agent workflow

```text
1. rbuilder discover .              # once per repo (or after large changes)
2. rbuilder -f json <command>      # compact facts on stdout
3. Parse schema_version + payload   # never scrape stderr for JSON
```

Set `REPO` to the repository root (where `.rbuilder/` lives):

```bash
export REPO=/path/to/repo
rbuilder -r "$REPO" -f json gql 'MATCH (n:Function) RETURN n LIMIT 20'
```

---

## High-value commands (low token cost)

| Intent | Command |
|--------|---------|
| Inventory functions | `rbuilder -f json gql --macro-name all_functions unused` |
| Find symbol by pattern | `rbuilder -f json gql "MATCH (n:Function) WHERE n.name LIKE '*Service*' RETURN n LIMIT 20"` |
| Natural-language function search | `rbuilder semantic index` then `rbuilder -f json semantic query "checkout flow" --limit 10` |
| Impact before editing | `rbuilder -f json blast-radius <Symbol> [--depth N]` |
| Architectural hotspots | `rbuilder -f json metrics --pagerank` |
| Call neighborhood | `rbuilder -f json gql "MATCH (a:Function)-[:CALLS*1..3]->(b:Function) RETURN a,b LIMIT 50"` |
| Migration plan | `rbuilder discover . --all --export-migration-plan` then read `.rbuilder/dashboard/migration_plan.json` |
| CI gate on changes | `rbuilder -f json check --policy-file policy.json` (exit 1 = violations) |

---

## Repeated queries in one session

**Option A — HTTP (recommended):**

```bash
rbuilder -r "$REPO" serve --open
# POST http://127.0.0.1:8080/api/query  {"query":"MATCH (n:Function) RETURN n LIMIT 5"}
```

See [docs/http-api.md](docs/http-api.md).

**Option B — Legacy socket daemon:**

```bash
rbuilder -r "$REPO" serve --daemon
# blast-radius auto-connects to .rbuilder/query.sock unless RBUILDER_NO_QUERY_DAEMON=1
```

---

## Rules of thumb

1. **Index first** — `gql`, `blast-radius`, `metrics` fail without `discover`.
2. **Use `-f json`** — stable `schema_version` fields; see [cli-output-schemas.md](docs/cli-output-schemas.md).
3. **`inspect` takes a symbol only** — no `--class` (use `blast-radius` for disambiguation).
4. **`slice --function`** is the **method/function name**, not the class name.
5. **`export --query`** uses filter syntax (`name:Foo`, `type:Function`, `all`) — not full GQL `MATCH`.
6. **Deep analysis** needs `discover --cfg` or `--all` (slice, inspect, taint).
7. **Semantic search** needs `semantic index` (separate from discover); default embedder requires LFS weights when building from source.
8. **Profile discover** — `discover -v` with `RUST_LOG=profile=info` for `[profile] stage` and centrality sub-phase timings (see [analysis-architecture.md](docs/analysis-architecture.md)).

---

## On-disk artifacts for agents

After `discover`:

| Path | Content |
|------|---------|
| `.rbuilder/graph.snapshot.bin` | Graph snapshot |
| `.rbuilder/dashboard/manifest.json` | Counts, feature flags |
| `.rbuilder/dashboard/migration_plan.json` | Migration export (with `--export-migration-plan`) |
| `.rbuilder/dashboard/graph_payload.bin` | Columnar graph for dashboard WASM |
| `.rbuilder/semantic_index.bin` | Opt-in semantic search index (`semantic index`) |

---

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Policy violation (`check`, `blast-radius --policy-file`) or command error |

---

## See also

- [Introduction](docs/Introduction.md) — concepts
- [User Guide](docs/user-guide.md) — full CLI
- [Further reading](docs/further-reading.md) — research map and contribution ideas
