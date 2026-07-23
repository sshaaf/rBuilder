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
| Release a version | [Releasing](releasing.md) · [GitHub Releases](https://github.com/sshaaf/rBuilder/releases/latest) |
| Research / papers behind features | [Further reading](further-reading.md) |
| FAQ / glossary | [FAQ](faq.md) · [Glossary](glossary.md) |
| Blast-radius caches and graph storage | [Graph storage architecture](graph-storage-architecture.md) · [CLI I/O sanity QE](cli-io-sanity-qe.md) |

## Quick paths

**First hour (CLI):** Introduction → User Guide §1–4 → `discover` on [ecommerce-java](user-guide.md#3-example-project-ecommerce-java) (includes CoolStore `/services/*`).

**First hour (dashboard):** User Guide §4 (`--with-dashboard`) → [Dashboard user guide](dashboard-user-guide.md) → `rbuilder serve --open`.

**Agent loop:** [AGENTS.md](../AGENTS.md) → `discover` once → `gql` / `blast-radius` with `-f json`.

Docs match the CLI in this repository — verify with `rbuilder --version`.

## Terminology

| Term | Meaning in rBuilder |
|------|---------------------|
| Tier 1 languages | **Nine** always-linked plugins: Rust, Python, JavaScript, TypeScript, Go, Java, C#, C, C++ |
| `--with-cfg` / `--cfg` | Same flag — CFG/PDG archive; prefer `--with-cfg` in docs |
| Communities | **Label propagation** (Raghavan 2007), not Louvain/Leiden; field `louvain_community_id` is historical |
| Dashboard / migration JSON | **Opt-in** via `--with-dashboard` / `--export-migration-hints` |
| `export` formats | `json`, `graphml`, `graphviz`, `mermaid` (not `dot`) |
| First-hour fixture | In-tree **ecommerce-java** (not external coolstore) |

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

- [cli-getting-started.md](cli-getting-started.md) — stub only; prefer [User Guide](user-guide.md) + ecommerce-java.

## Internal notes

Maintainer drafts and scratch notes live under [`internal/`](internal/) (not part of the public doc set).
