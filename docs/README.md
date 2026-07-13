# rBuilder documentation

Start here based on what you need.

## I want to…

| Goal | Start here |
|------|------------|
| Understand what rBuilder is and how the graph works | [Introduction](Introduction.md) · [Feature designs](design/README.md) |
| Install, index a repo, run CLI commands | [User Guide](user-guide.md) |
| Use the browser dashboard (tabs, graphs, migration) | [Dashboard user guide](dashboard-user-guide.md) |
| Integrate an LLM agent or automation | [AGENTS.md](../AGENTS.md) · [JSON API](json-api.md) · [Agent recipes](agent-recipes.md) |
| Call `rbuilder serve` over HTTP | [HTTP API](http-api.md) |
| Plan a monolith migration | [Building a migration plan](building-migration-plan.md) · [Migration planner design](design/migration-planner-design.md) |
| Feature engineering designs (screenshots) | [design/](design/README.md) |
| See supported languages | [Language guide](LANGUAGE_GUIDE.md) |
| Parse exact JSON field names | [CLI output schemas](cli-output-schemas.md) |
| Write a blast-radius CI policy | [Policy format](policy-format.md) |
| Contribute code or languages | [CONTRIBUTING.md](../CONTRIBUTING.md) · [Tier 1 language support](tier-1-language-support.md) · [Code structure](Code_structure.md) |
| Release a version | [Releasing](releasing.md) |
| Research / papers behind features | [Further reading](further-reading.md) |
| Blast-radius caches and graph storage | [Graph storage architecture](graph-storage-architecture.md) · [CLI I/O sanity QE](cli-io-sanity-qe.md) |

## Quick paths

**First hour (CLI):** Introduction → User Guide §1–4 → `discover` on [coolstore](user-guide.md#3-example-project-coolstore).

**First hour (dashboard):** User Guide §4 → [Dashboard user guide](dashboard-user-guide.md) → open `.rbuilder/dashboard/`.

**Agent loop:** [AGENTS.md](../AGENTS.md) → `discover` once → `gql` / `blast-radius` with `-f json`.

## Engineering docs (maintainers)

| Document | Topic |
|----------|--------|
| [Feature designs](design/README.md) | Per-capability engineering docs + dashboard screenshots |
| [Dashboard design](dashboard-design.md) | WASM export pipeline, phases |
| [Analysis architecture](analysis-architecture.md) | CFG / PDG / taint crates |
| [Graph storage architecture](graph-storage-architecture.md) | Snapshots, columnar v2, blast lookup cache |
| [CLI I/O sanity QE](cli-io-sanity-qe.md) | Golden-path test contract and `blast_radius_perf` gates |
| [Harmonic centrality](harmonic-centrality.md) | Migration metric detail |
| [Migration algorithms](migration-algorithms.md) | Ordering math |

## Deprecated / duplicate entry points

- [cli-getting-started.md](cli-getting-started.md) — shortened coolstore walkthrough; prefer [User Guide](user-guide.md) for the canonical reference.

## Internal notes

Maintainer drafts and scratch notes live under [`internal/`](internal/) (not part of the public doc set).
