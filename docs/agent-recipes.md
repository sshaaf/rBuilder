# Agent recipes

Copy-paste workflows for LLM agents and automation. All commands assume:

```bash
export REPO=/path/to/repo   # contains .rbuilder/ after discover
```

**JSON shapes:** [json-api.md](json-api.md) · **Field tables:** [cli-output-schemas.md](cli-output-schemas.md)

---

## Recipe 1 — Orient in an unfamiliar repo

```bash
rbuilder -r "$REPO" discover .
rbuilder -r "$REPO" -f json discover . | jq '.metrics'
rbuilder -r "$REPO" -f json gql --macro-name all_functions unused | jq '.count'
rbuilder -r "$REPO" -f json metrics --pagerank | jq '.rows[:10]'
```

**Use when:** first turn on a codebase; replaces reading directory trees.

---

## Recipe 2 — Before editing a symbol

```bash
SYMBOL=ShoppingCartService
rbuilder -r "$REPO" -f json blast-radius "$SYMBOL" | jq '{
  score: .metrics.score,
  direct_callers: .metrics.direct_caller_count,
  impact_zone: .metrics.impact_zone_count
}'
rbuilder -r "$REPO" -f json blast-radius "$SYMBOL" --depth 3 | jq '.topology.callers[:10]'
```

If the name is ambiguous, disambiguate:

```bash
rbuilder -r "$REPO" blast-radius process --class ShoppingCartService
```

**Use when:** agent plans a refactor or bugfix; avoids missing upstream callers.

---

## Recipe 3 — Find entrypoints / APIs

```bash
rbuilder -r "$REPO" -f json gql \
  "MATCH (n:Function) WHERE n.name LIKE '*Endpoint' RETURN n LIMIT 20" \
  | jq '.rows[].n.name'
```

**Use when:** tracing HTTP handlers or CLI entrypoints.

---

## Recipe 3b — Natural-language function discovery

```bash
rbuilder -r "$REPO" semantic index
rbuilder -r "$REPO" -f json semantic query "shopping cart checkout" --limit 10 \
  | jq '.hits[] | {name, file_path, score: .fused_score}'
rbuilder -r "$REPO" -f json semantic query "OrderService validate" --keyword-and --fusion \
  | jq '.hits[:5]'
```

**Use when:** the agent knows intent but not exact symbol names; complements GQL `LIKE` patterns.

---

## Recipe 4 — Call chain neighborhood

```bash
rbuilder -r "$REPO" -f json gql \
  "MATCH (a:Function)-[:CALLS*1..3]->(b:Function) RETURN a,b LIMIT 50"
```

**Use when:** understanding feature locality without opening every file.

---

## Recipe 5 — Data-flow check at a line (needs `discover --cfg`)

```bash
rbuilder -r "$REPO" discover . --cfg
rbuilder -r "$REPO" -f json slice \
  src/main/java/com/example/Service.java \
  --line 42 --variable request --function handleRequest \
  | jq '.lines'
```

Note: `--function` is the **method name**, not the class name.

**Use when:** verifying what affects a variable before changing logic.

---

## Recipe 6 — Taint sanity check

```bash
rbuilder -r "$REPO" discover . --cfg
rbuilder -r "$REPO" -f json slice src/.../Controller.java \
  --line 30 --variable param --function handle --taint | jq '.flows'
```

**Use when:** security-sensitive edits (user input → sink).

---

## Recipe 7 — Migration batch planning

```bash
rbuilder discover . --all --export-migration-plan
jq '.packages[:10]' "$REPO/.rbuilder/dashboard/migration_plan.json"
rbuilder serve --open   # Migration tab for interactive tuning
```

**Use when:** monolith extraction ordering for humans or agents.

---

## Recipe 8 — CI policy on a branch

```bash
cp docs/examples/policy-strict.json policy.json
rbuilder -r "$REPO" -f json check --policy-file policy.json
# exit 1 → violations in .violations[]
```

**Use when:** blocking PRs that touch high-impact symbols.

---

## Recipe 9 — HTTP session (many queries)

```bash
rbuilder -r "$REPO" serve &
curl -sS -X POST http://127.0.0.1:8080/api/query \
  -H 'Content-Type: application/json' \
  -d '{"query":"MATCH (n:Function) RETURN n LIMIT 5"}' | jq '.count'
```

See [http-api.md](http-api.md).

---

## Recipe 10 — Export subgraph for external tools

```bash
# Filter syntax (not GQL MATCH):
rbuilder -r "$REPO" export --export-format graphml \
  --export-output service.graphml --query "name:ShoppingCartService"
rbuilder -r "$REPO" export --export-format mermaid \
  --export-output all-calls.mmd --query all
```

**Use when:** handing a neighborhood to GraphML/Gephi or docs.

---

## See also

- [AGENTS.md](../AGENTS.md)
- [User Guide](user-guide.md)
