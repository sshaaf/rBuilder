# rBuilder Agent Skills

Invocable skill bundles for AI coding agents — workflows they activate when a task matches, instead of reading static docs every turn.

Skills follow the [agentskills.io](https://agentskills.io/specification) open standard and work across Claude Code, Cursor, OpenCode, Goose, and any compatible agent.

---

## Quick start

1. **Install rBuilder** — [GitHub Releases](https://github.com/sshaaf/rBuilder/releases) or `cargo build --release`
2. **Index your repo** — `rbuilder discover .`
3. **Install skill bundles** for your agent runtime (see [Platform install](#platform-install) below)

---

## Skill index

| Skill | When to use | Key commands |
|-------|-------------|--------------|
| [rbuilder-orient](skills/rbuilder-orient/SKILL.md) | Explore unfamiliar repo, understand structure | `discover`, `gql`, `metrics --pagerank`, `semantic query` |
| [rbuilder-impact](skills/rbuilder-impact/SKILL.md) | Refactor, rename, delete — change impact analysis | `blast-radius`, `gql` caller queries |
| [rbuilder-session](skills/rbuilder-session/SKILL.md) | Multiple queries in one task, HTTP session | `serve`, `/api/query`, `/api/semantic/query` |
| [rbuilder-neighborhood](skills/rbuilder-neighborhood/SKILL.md) | Call chains, dependency graphs, export subgraphs | `gql` CALLS traversal, `export` |
| [rbuilder-slice](skills/rbuilder-slice/SKILL.md) | Line-level data flow, variable tracking | `slice` with `--view text/cfg/pdg` |
| [rbuilder-security](skills/rbuilder-security/SKILL.md) | Taint analysis, source-to-sink security flows | `slice --taint` |
| [rbuilder-migration](skills/rbuilder-migration/SKILL.md) | Migration planning, batch refactor strategy | `discover --export-migration-hints` |
| [rbuilder-ci-gate](skills/rbuilder-ci-gate/SKILL.md) | CI policy enforcement, blast radius limits | `check --policy-file` |

---

## Platform install

### Claude Code

Add to `.claude/settings.json` in your project root:

```json
{
  "permissions": {
    "allow": [
      "Bash(rbuilder:*)"
    ]
  },
  "skills": [
    "skills/rbuilder-orient",
    "skills/rbuilder-impact",
    "skills/rbuilder-session",
    "skills/rbuilder-neighborhood",
    "skills/rbuilder-slice",
    "skills/rbuilder-security",
    "skills/rbuilder-migration",
    "skills/rbuilder-ci-gate"
  ]
}
```

### Cursor

Copy or symlink each skill directory into `.cursor/skills/`:

```bash
mkdir -p .cursor/skills
for skill in skills/rbuilder-*/; do
  ln -s "../../$skill" ".cursor/skills/$(basename $skill)"
done
```

### OpenCode

Copy or symlink each skill directory into `.opencode/skills/`:

```bash
mkdir -p .opencode/skills
for skill in skills/rbuilder-*/; do
  ln -s "../../$skill" ".opencode/skills/$(basename $skill)"
done
```

### Goose

Copy or symlink each skill directory into `.goose/skills/`:

```bash
mkdir -p .goose/skills
for skill in skills/rbuilder-*/; do
  ln -s "../../$skill" ".goose/skills/$(basename $skill)"
done
```

### Generic fallback

If your agent does not support skill bundles, paste `AGENTS.md` into your agent's system prompt and refer to `docs/agent-recipes.md` for copy-paste workflows.

---

## Further reading

- [AGENTS.md](AGENTS.md) — universal agent rules (discover once, use `-f json`, parse stdout only)
- [docs/agent-recipes.md](docs/agent-recipes.md) — 10 copy-paste agent recipes
- [docs/json-api.md](docs/json-api.md) — full JSON schema reference for all commands
- [docs/http-api.md](docs/http-api.md) — HTTP server API (`serve`)
