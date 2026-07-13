# Releasing rBuilder

How maintainers publish versioned binaries and GitHub Releases.

---

## Version numbers

- **Crate / CLI version** lives in root [`Cargo.toml`](../Cargo.toml) (`[package].version`).
- **Workspace crates** share the same version in their `Cargo.toml` files and `[workspace.dependencies]` pins.
- **Git tags** use a `v` prefix: `v0.2.0` (not `0.2.0` alone).

Bump all workspace versions together before tagging.

---

## Release workflow (automated)

Pushing a tag matching `v*` triggers [`.github/workflows/release.yml`](../.github/workflows/release.yml):

1. **Build** `rbuilder` release binaries for:
   - `x86_64-unknown-linux-gnu`
   - `aarch64-apple-darwin`
   - `x86_64-apple-darwin`
   - `x86_64-pc-windows-msvc`
2. **Package** as `rbuilder-<version>-<target>.tar.gz` (or `.zip` on Windows).
3. **Publish** a GitHub Release with auto-generated notes and `SHA256SUMS.txt`.

### Tag and push

```bash
# On main, with a clean tree and versions already bumped
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0
```

Track the run: **Actions → Release**.

### Manual re-run

From the Actions tab, run **Release** via **workflow_dispatch** with:

- `tag`: e.g. `v0.2.0`
- `ref`: branch or SHA to build (default `main`)
- `draft`: optional draft release

---

## Pre-release checks (local)

```bash
cargo build --release
cargo test --release

# Dashboard asset build (if UI changed)
cd dashboard && npm ci && npm run build && cd ..

# Optional: golden repo validation
./scripts/validate-golden-repos.sh
```

---

## Assets users download

From [GitHub Releases](https://github.com/sshaaf/rBuilder/releases):

| Platform | Asset pattern |
|----------|----------------|
| macOS Apple Silicon | `rbuilder-*-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `rbuilder-*-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 | `rbuilder-*-x86_64-unknown-linux-gnu.tar.gz` |
| Windows | `rbuilder-*-x86_64-pc-windows-msvc.zip` |

Extract and run `rbuilder --version`. See [User Guide §1](user-guide.md#1-installation).

---

## After release

- Verify the Release page lists all four platform archives and checksums.
- Smoke-test `discover` + `gql` on a small repo with the downloaded binary.
- If `RBUILDER_TESTS_DISPATCH_TOKEN` is configured, CI dispatches `rbuilder-released` to the external test repo (see workflow comments).

---

## See also

- [CONTRIBUTING.md](../CONTRIBUTING.md) — development setup
- [User Guide](user-guide.md) — install from release artifacts
