# Language guide

rBuilder indexes source and config files through **language plugins**. This guide lists what ships in the binary and how depth of analysis varies by language.

**Contributor checklist for new Tier 1 languages:** [tier-1-language-support.md](tier-1-language-support.md)

---

## Tiers

| Tier | Handler | Indexing | CFG / PDG / taint | Typical use |
|------|---------|----------|-------------------|-------------|
| **Tier 1** | Custom `LanguagePlugin` | Rich symbols + `Calls` edges | Full pipeline when profile enabled | Production repos |
| **Tier 2** | Generic tree-sitter | Kinds from `LanguageConfig` | Limited | Broader syntax coverage |
| **Tier 3** | Regex | Pattern-based symbols | None | Config / glue files |

Source of truth for Tier 1 metadata: [`languages.toml`](../languages.toml) at the repo root.

---

## Tier 1 languages (always in the release binary)

These nine use dedicated tree-sitter plugins and custom extractors:

| Language | Extensions | CFG / PDG (`discover --cfg`) | Taint | Notes |
|----------|------------|------------------------------|-------|-------|
| **Java** | `.java` | ✅ Full | ✅ Rich patterns | Best golden-repo coverage |
| **Go** | `.go` | ✅ Full | ✅ Rich patterns | Strong dashboard gates |
| **Rust** | `.rs` | ✅ | ✅ | |
| **Python** | `.py`, `.pyw` | ✅ | ✅ | |
| **JavaScript** | `.js`, `.jsx`, `.mjs` | ✅ | ✅ | |
| **TypeScript** | `.ts`, `.tsx` | ✅ | ✅ | |
| **C#** | `.cs` | ✅ | ✅ | |
| **C** | `.c`, `.h` | ✅ | ✅ | |
| **C++** | `.cpp`, `.cc`, `.cxx`, `.hpp`, … | ✅ | ✅ | |

Filter at discover time:

```bash
rbuilder discover . -l java,go,rust
rbuilder discover . -e node_modules,target,.git
```

---

## Config, docs, and IaC (additional plugins)

Beyond Tier 1, rBuilder registers plugins for common **config and markup** formats (JSON, YAML, TOML, properties, Markdown, CI YAML, Ansible, Chef, Puppet, and related paths). These contribute **config and structure nodes** to the graph; they do not run the CFG/PDG pipeline.

Exact plugin set evolves with releases — search `crates/rbuilder-config-formats` and `crates/rbuilder-lang-*` for the current list.

---

## Choosing discover depth

| Command | When to use |
|---------|-------------|
| `discover .` | Fast graph + metrics + dashboard (default) |
| `discover . --security` | Add secret scanning on config-like files |
| `discover . --cfg` | Per-function CFG, PDG, slice, inspect, taint |
| `discover . --with-cfg --with-security --with-taint` | Full analysis + migration export + complete dashboard bundles |

CFG analysis is **much slower** on large repos (tens of thousands of functions). Run `discover . --cfg` on a small sample first; golden-repo timing checks live in `tests/discover_perf_baselines.rs` (manual, `#[ignore]`).

---

## Symbol and CLI tips

- **GQL** and **blast-radius** use graph node names from indexing (often bare method names in Java).
- **`inspect SYMBOL`** takes a **function symbol** only (no `--class`). Use a unique name or disambiguate via GQL first.
- **`blast-radius`** supports `--class` and `--file` when names collide.
- **`slice --function`** must be the **function/method name** in the source file (not the enclosing class name).

---

## See also

- [User Guide §4 — discover](user-guide.md#4-index-with-discover)
- [Introduction — indexing](Introduction.md#indexing-the-repository-discover)
- [Tier 1 language support](tier-1-language-support.md) — requirements for contributors
