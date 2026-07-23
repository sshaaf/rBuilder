# rBuilder CLI Getting Started

> **This page is deprecated.** Use the canonical walkthrough instead.

**Canonical first hour:** [User Guide §1–4](user-guide.md#1-installation) on the in-tree **[ecommerce-java](user-guide.md#3-example-project-ecommerce-java)** fixture.

| Goal | Doc |
|------|-----|
| Install + PATH | [User Guide §1–2](user-guide.md#1-installation) |
| Index with `discover` | [User Guide §4](user-guide.md#4-index-with-discover) |
| Concepts | [Introduction](Introduction.md) |
| Agents / `-f json` | [AGENTS.md](../AGENTS.md) · [Agent recipes](agent-recipes.md) |
| Dashboard | [Dashboard user guide](dashboard-user-guide.md) |

**Note:** Dashboard and migration JSON are **opt-in** (`discover --with-dashboard`, `--export-migration-hints`). There is no `--all` flag — combine `--with-cfg --with-security --with-taint` explicitly when you need the deep pass.
