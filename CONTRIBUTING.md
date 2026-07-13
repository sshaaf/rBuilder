# Contributing to rBuilder

Thanks for helping improve rBuilder. This guide covers local setup, tests, and where to put changes.

**Documentation map:** [docs/README.md](docs/README.md)

---

## Prerequisites

- **Rust** stable (via [rustup](https://rustup.rs/))
- **Node.js 18+** and npm (dashboard UI only)
- **git**

Optional: **Playwright** (dashboard browser tests) — installed via `dashboard/` npm scripts.

---

## Clone and build

```bash
git clone https://github.com/sshaaf/rBuilder.git
cd rBuilder
cargo build --release
./target/release/rbuilder --version
```

### Dashboard (when changing `dashboard/`)

```bash
cd dashboard
npm ci
npm run build
cd ..
cargo build --release   # embeds dashboard/dist
```

WASM worker:

```bash
# from repo root — see dashboard/wasm/ or project scripts if present
cargo build -p rbuilder-wasm --target wasm32-unknown-unknown --release
```

---

## Running tests

```bash
# Unit / integration (workspace)
cargo test

# Release-mode CLI golden paths (slower)
cargo test --release --test subprocess_golden_path
cargo test --release --test all_commands_sanity

# Dashboard bundle assertions
cargo test dashboard_harness

# Golden repos (optional, long)
./scripts/validate-golden-repos.sh
# Discover timing baselines (manual): cargo test --release --test discover_perf_baselines -- --ignored --nocapture
```

### Dashboard Playwright scripts

Serve a discovered dashboard, then:

```bash
cd dashboard
DASHBOARD_URL=http://127.0.0.1:8765/ node scripts/test-guide-cli.mjs
```

---

## Project layout (short)

| Area | Path |
|------|------|
| CLI entry | `src/cli/` |
| Analysis (CFG, PDG, taint) | `crates/rbuilder-analysis/` |
| Graph storage | `crates/rbuilder-graph/` |
| Dashboard export | `crates/rbuilder-dashboard/` |
| Browser UI | `dashboard/src/` |
| WASM engine | `crates/rbuilder-wasm/` |
| Language plugins | `crates/rbuilder-lang-*/` |

Full map: [docs/Code_structure.md](docs/Code_structure.md)

---

## Adding or improving a language

Follow [docs/tier-1-language-support.md](docs/tier-1-language-support.md) and update [docs/LANGUAGE_GUIDE.md](docs/LANGUAGE_GUIDE.md).

---

## Documentation changes

- **User-facing:** `docs/Introduction.md`, `docs/user-guide.md`, `docs/dashboard-user-guide.md`
- **Agents:** `AGENTS.md`, `docs/json-api.md`, `docs/agent-recipes.md`
- **Accuracy:** keep CLI examples aligned with `dashboard/scripts/validate-guide-cli-gbuilder.sh` where possible

---

## Pull requests

1. Branch from `main` (or the active integration branch).
2. Keep commits focused; match existing Rust style and `cargo fmt` / `clippy` expectations.
3. CI runs on PRs when a maintainer adds the **`ci`** label (see [.github/workflows/ci.yml](.github/workflows/ci.yml)).
4. Fill in the PR template with test commands you ran.

---

## Releases

Maintainers: [docs/releasing.md](docs/releasing.md)
