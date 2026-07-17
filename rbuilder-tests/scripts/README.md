# Scripts

## `run_rbuilder_report.py`

Runs the full rBuilder feature matrix against all Tier 1 `ecommerce-*` apps and publishes reports under `rbuilder-reports/`.

### Outputs

| File | Description |
|------|-------------|
| `rbuilder-reports/REPORT.md` | Cross-project **summary** report |
| `rbuilder-reports/REPORT.html` | HTML summary |
| `rbuilder-reports/languages/<id>.md` | **Comprehensive** per-language report |
| `rbuilder-reports/languages/<id>.html` | Per-language HTML report |
| `rbuilder-reports/README.md` | Index linking all artifacts |
| `rbuilder-reports/all-results.json` | Combined JSON results |
| `rbuilder-reports/<id>-summary.json` | Per-project summaries |
| `rbuilder-reports/<id>-metrics.json` | Raw metrics output |
| `rbuilder-reports/<id>-blast.json` | Raw blast-radius output (checkout target) |
| `rbuilder-reports/<id>-blast-top.json` | Top blast scores from full function scan |
| `rbuilder-reports/<id>-export.json` | Exported function subgraph |

### Usage

```bash
# from rbuilder-tests/ (auto-detect: PATH, RBUILDER, or ../../target/{release,debug}/rbuilder when embedded)
./scripts/run_rbuilder_report.sh

# explicit binary + refresh README summary tables
RBUILDER=/path/to/rbuilder ./scripts/run_rbuilder_report.py --update-readmes

# subset of projects, keep existing .rbuilder caches
./scripts/run_rbuilder_report.py --projects rust java --no-clean
```

### Options

| Flag | Description |
|------|-------------|
| `--rbuilder PATH` | rbuilder binary |
| `--output-dir PATH` | default: `rbuilder-reports/` |
| `--repo-root PATH` | default: parent of `scripts/` |
| `--no-clean` | skip deleting `.rbuilder/` before discover |
| `--update-readmes` | sync summary tables into root + project READMEs |
| `--projects rust python …` | run subset only |
| `--blast-top N` | keep top N blast scores per project (default: 10) |
| `--skip-blast-scan` | skip full function blast scan (faster; omits top scores) |

Exit code **0** if every project `discover` succeeds; **1** otherwise.

### Graph correctness

Hand-labeled facts live in `ecommerce-*/correctness/expected-facts.json` (see [`correctness/SCHEMA.md`](../correctness/SCHEMA.md)). They are checked by `cargo test --test graph_correctness` in the rBuilder repo (not this report script).

### Install rbuilder from GitHub Releases

[`install_rbuilder_release.sh`](install_rbuilder_release.sh) downloads the platform archive published by [rBuilder releases](https://github.com/sshaaf/rbuilder/releases):

```bash
./scripts/install_rbuilder_release.sh
RBUILDER_TAG=v0.1.0 ./scripts/install_rbuilder_release.sh
RBUILDER=/path/to/.rbuilder-bin/rbuilder ./scripts/run_rbuilder_report.py
```

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `RBUILDER_REPO` | `sshaaf/rbuilder` | GitHub `owner/repo` |
| `RBUILDER_TAG` | _(latest)_ | Release tag, e.g. `v0.1.0` |
| `RBUILDER_TARGET` | auto-detect | Rust triple, e.g. `x86_64-unknown-linux-gnu` |
| `RBUILDER_INSTALL_DIR` | `.rbuilder-bin/` | Extract destination |
| `GITHUB_TOKEN` / `GH_TOKEN` | — | Optional; higher API rate limits / private release assets |

### GitHub Actions

[`.github/workflows/rbuilder-report.yml`](../.github/workflows/rbuilder-report.yml) runs when:

1. **rBuilder publishes a release** — the [rBuilder release workflow](https://github.com/sshaaf/rbuilder/blob/main/.github/workflows/release.yml) sends a `repository_dispatch` (`rbuilder-released`) with the new tag.
2. **Manual run** — Actions → **rBuilder Report** → Run workflow.

The workflow:

1. Downloads the matching rbuilder binary from GitHub Releases
2. Runs `./scripts/run_rbuilder_report.py --no-clean`
3. Packages `rbuilder-reports/` as **`rbuilder-reports-<tag>-<run_id>.tar.gz`** (+ SHA256 sidecar)
4. Uploads the archive as a **workflow artifact** (90-day retention) — nothing is committed to git

#### Setup (one-time)

In **sshaaf/rBuilder** repository secrets:

| Secret | Purpose |
|--------|---------|
| `RBUILDER_TESTS_DISPATCH_TOKEN` | PAT (classic `repo` or fine-grained **Actions: write** on `rbuilder-tests`) to trigger the report workflow |

Optional in **sshaaf/rbuilder-tests**:

| Secret | Purpose |
|--------|---------|
| `RBUILDER_DOWNLOAD_TOKEN` | Only if rBuilder releases are private |

#### Download a report

Open the workflow run → **Artifacts** → download `rbuilder-reports-<tag>-<run_id>.tar.gz`.

```bash
tar -xzf rbuilder-reports-v0.1.0-123456789.tar.gz
open rbuilder-reports/REPORT.html
```

### Requirements

- `rbuilder` built with language bundles (uses `discover . --cfg`)
- [`rbuilder-policy.json`](../rbuilder-policy.json) at repo root (for `check`)
