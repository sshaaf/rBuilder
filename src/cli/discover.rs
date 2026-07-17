//! `rbuilder discover` — index and analyze a repository.

use super::context::CliContext;
use anyhow::Result;

pub struct DiscoverArgs {
    pub path: Option<String>,
    pub languages: Option<String>,
    pub exclude: Option<String>,
    pub security: bool,
    pub cfg: bool,
    pub all: bool,
    /// Also write legacy JSON graph files (`graph.db` / `graph.json`).
    pub write_json_graph: bool,
    /// Export `.rbuilder/dashboard/` bundle. Default off.
    pub with_dashboard: bool,
    /// Write a migration roadmap JSON after analysis completes.
    pub export_migration_hints: bool,
    /// Compute harmonic centrality (HyperBall on large graphs). Default off.
    pub with_harmonic: bool,
    /// Preset strategy for `--export-migration-hints` (default: hybrid_default).
    pub migration_preset: String,
    /// Roadmap row order: `scheduled` (deps) or `priority` (score rank).
    pub migration_order: String,
}

pub fn run(ctx: &CliContext, args: DiscoverArgs) -> Result<()> {
    let path = args
        .path
        .as_deref()
        .map(|p| {
            if std::path::Path::new(p).is_absolute() {
                p.to_string()
            } else {
                ctx.repo.join(p).to_string_lossy().into_owned()
            }
        })
        .unwrap_or_else(|| ctx.repo.to_string_lossy().into_owned());

    super::discover_impl::run_full_analysis(
        ctx,
        &path,
        args.languages,
        args.exclude,
        args.security,
        args.cfg,
        args.all,
        args.write_json_graph,
        args.with_dashboard,
        args.export_migration_hints,
        args.with_harmonic,
        &args.migration_preset,
        &args.migration_order,
        &ctx.db,
    )
}
