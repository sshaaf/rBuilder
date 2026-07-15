//! `rbuilder semantic` — opt-in function semantic index and Hamming search.

use super::args::OutputFormat;
use super::context::CliContext;
use super::semantic_output::{
    build_index_response, build_query_response, hit_from_semantic, index_response_to_json,
    query_response_to_json,
};
use crate::analysis::{
    blast_summary_from_result, build_index, default_model_path, default_tokenizer_path,
    expand_semantic_hits, query_index_with_fusion, resolve_embedder, try_load_engine,
    validate_mrl_dimensions, AnalysisResults, BlastRadiusEngine, BlastSummaryProvider,
    EmbedderChoice, OnnxReloadOptions, SemanticBuildOptions, SemanticExpandConfig,
    SemanticExpandMode, SemanticExpansion, SemanticFusionConfig, SemanticIndex,
    CODE_DAEMON_MRL_DIMS,
};
use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use crate::graph::backend::{GraphBackend, MemoryBackend};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum CliEmbedderKind {
    /// Deterministic sign-hash (no model files).
    Hash,
    /// Generic ONNX `--model` (hash tokenization, or `--tokenizer` for SentencePiece).
    Onnx,
    /// [`code-daemon-embed-v1`](https://huggingface.co/faxenoff/code-daemon-embed-v1) code retriever.
    CodeDaemon,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum CliExpandMode {
    Neighbors,
    Blast,
    Gql,
    All,
}

pub struct SemanticIndexArgs {
    pub dimensions: usize,
    pub incremental: bool,
    pub embedder: CliEmbedderKind,
    pub model: Option<PathBuf>,
    pub tokenizer: Option<PathBuf>,
}

pub struct SemanticQueryArgs {
    pub query: String,
    pub limit: usize,
    pub expand: Option<CliExpandMode>,
    pub expand_depth: usize,
    pub model: Option<PathBuf>,
    pub tokenizer: Option<PathBuf>,
    pub fusion: bool,
    pub candidate_pool: usize,
    pub keyword_and: bool,
}

pub fn run_index(ctx: &CliContext, args: SemanticIndexArgs) -> Result<()> {
    if args.dimensions == 0 || args.dimensions % 8 != 0 {
        bail!("--dimensions must be a positive multiple of 8");
    }
    if args.embedder == CliEmbedderKind::Onnx && args.model.is_none() {
        bail!("--model is required when --embedder onnx");
    }

    let graph = ctx.load_graph()?;
    let graph_digest = ctx.graph_digest()?;
    let path = semantic_index_path(ctx);

    let existing = if args.incremental && path.is_file() {
        Some(SemanticIndex::load(&path).with_context(|| format!("load {}", path.display()))?)
    } else {
        None
    };

    let (model_path, embedder_choice) = resolve_embedder_choice(&ctx.repo, &args)?;
    if args.embedder == CliEmbedderKind::CodeDaemon {
        validate_mrl_dimensions(args.dimensions).with_context(|| {
            format!(
                "code-daemon supports MRL dims {:?} (multiples of 8, max 768)",
                CODE_DAEMON_MRL_DIMS
            )
        })?;
    }

    let embedder = resolve_embedder(&embedder_choice, args.dimensions)?;

    let (stored_model, stored_tokenizer) =
        store_paths(&ctx.repo, &embedder_choice, &model_path, &args.tokenizer);

    let (index, stats) = build_index(
        graph.backend(),
        embedder.as_ref(),
        SemanticBuildOptions {
            dimensions: args.dimensions,
            graph_digest,
            incremental: args.incremental,
            existing,
            model_path: stored_model,
            tokenizer_path: stored_tokenizer,
            repo_root: Some(ctx.repo.clone()),
        },
    )?;

    index
        .save(&path)
        .with_context(|| format!("write semantic index {}", path.display()))?;

    let response = build_index_response(
        &index.model_id,
        index.dimensions,
        index.len(),
        &path.display().to_string(),
        index.graph_digest.clone(),
        Some(stats),
    );

    if ctx.format == OutputFormat::Json {
        ctx.emit_json_value(&index_response_to_json(&response))?;
    } else {
        println!(
            "Indexed {} functions ({}, {} dims) → {}",
            response.functions_indexed,
            response.model_id,
            response.dimensions,
            response.path
        );
        if let Some(build_stats) = &response.build_stats {
            println!(
                "  incremental: {} reused, {} embedded, {} removed",
                build_stats.reused, build_stats.embedded, build_stats.removed
            );
        }
    }

    Ok(())
}

pub fn run_query(ctx: &CliContext, args: SemanticQueryArgs) -> Result<()> {
    let path = semantic_index_path(ctx);
    if !path.is_file() {
        bail!(
            "Semantic index not found at {} (run `rbuilder semantic index` first)",
            path.display()
        );
    }

    let index = SemanticIndex::load(&path)
        .with_context(|| format!("load semantic index {}", path.display()))?;

    let reload = OnnxReloadOptions {
        model_path: args.model.clone(),
        tokenizer_path: args.tokenizer.clone(),
    };

    let analysis_path = ctx.repo.join(".rbuilder/analysis_results.bin");
    let analysis = if analysis_path.is_file() {
        Some(
            AnalysisResults::load(&analysis_path).with_context(|| {
                format!("load analysis results {}", analysis_path.display())
            })?,
        )
    } else {
        None
    };

    let fusion = SemanticFusionConfig {
        enabled: args.fusion,
        candidate_pool: args.candidate_pool.max(args.limit),
        keyword_and: args.keyword_and,
        ..SemanticFusionConfig::default()
    };

    let hits = query_index_with_fusion(
        &index,
        &args.query,
        args.limit,
        &reload,
        &fusion,
        analysis.as_ref(),
        Some(&ctx.repo),
    )?;

    let graph = ctx.load_graph()?;
    let backend = graph.backend();

    let expansion = if let Some(mode) = args.expand {
        let expand_mode = match mode {
            CliExpandMode::Neighbors => SemanticExpandMode::Neighbors,
            CliExpandMode::Blast => SemanticExpandMode::Blast,
            CliExpandMode::Gql => SemanticExpandMode::Gql,
            CliExpandMode::All => SemanticExpandMode::All,
        };
        let config = SemanticExpandConfig {
            mode: expand_mode,
            call_depth: args.expand_depth.max(1),
            anchor_limit: args.limit.min(5),
            per_anchor_limit: 20,
        };
        let blast_provider = EngineBlastProvider {
            repo: &ctx.repo,
            backend,
            graph_digest: ctx.graph_digest()?,
        };
        let mut expansion = expand_semantic_hits(
            backend,
            &hits,
            &config,
            if matches!(expand_mode, SemanticExpandMode::Blast | SemanticExpandMode::All) {
                Some(&blast_provider)
            } else {
                None
            },
        )?;

        if matches!(expand_mode, SemanticExpandMode::Gql | SemanticExpandMode::All) {
            expansion.gql = Some(expand_gql_neighbors(
                backend,
                &hits,
                args.expand_depth.max(1),
                config.anchor_limit,
            )?);
        }
        Some(expansion)
    } else {
        None
    };

    let hit_json: Vec<_> = hits
        .iter()
        .map(|hit| hit_from_semantic(&hit.entry, hit.distance, index.dimensions, Some(hit)))
        .collect();

    let response = build_query_response(
        &args.query,
        &index.model_id,
        index.dimensions,
        hit_json,
        expansion,
    );

    if ctx.format == OutputFormat::Json {
        ctx.emit_json_value(&query_response_to_json(&response))?;
    } else {
        if response.hits.is_empty() {
            println!("No matches for {:?}", args.query);
            return Ok(());
        }
        for hit in &response.hits {
            let label = hit
                .qualified_name
                .as_deref()
                .unwrap_or(&hit.name);
            let file = hit
                .file_path
                .as_deref()
                .map(|p| format!(" ({p})"))
                .unwrap_or_default();
            println!(
                "{label}{file}  distance={} score={:.3}{}",
                hit.distance,
                hit.score,
                hit.fused_score
                    .map(|score| format!(" fused={score:.3}"))
                    .unwrap_or_default()
            );
        }
        if let Some(exp) = &response.expansion {
            print_expansion_text(exp);
        }
    }

    Ok(())
}

fn resolve_embedder_choice(
    repo: &Path,
    args: &SemanticIndexArgs,
) -> Result<(PathBuf, EmbedderChoice)> {
    match args.embedder {
        CliEmbedderKind::Hash => Ok((PathBuf::new(), EmbedderChoice::SignHash)),
        CliEmbedderKind::Onnx => {
            let model = args.model.clone().expect("checked");
            Ok((
                model.clone(),
                EmbedderChoice::Onnx {
                    model,
                    tokenizer: args.tokenizer.clone(),
                },
            ))
        }
        CliEmbedderKind::CodeDaemon => {
            let model = args
                .model
                .clone()
                .filter(|p| p.is_file())
                .unwrap_or_else(|| default_model_path(repo));
            if !model.is_file() {
                bail!(
                    "code-daemon model not found at {}.\nDownload with:\n  huggingface-cli download faxenoff/code-daemon-embed-v1 model_int8qdt.onnx sentencepiece.bpe.model --local-dir {}",
                    model.display(),
                    default_model_path(repo).parent().unwrap().display()
                );
            }
            Ok((
                model.clone(),
                EmbedderChoice::CodeDaemon {
                    model,
                    tokenizer: args.tokenizer.clone(),
                },
            ))
        }
    }
}

fn store_paths(
    repo: &Path,
    choice: &EmbedderChoice,
    _model_path: &Path,
    explicit_tokenizer: &Option<PathBuf>,
) -> (Option<String>, Option<String>) {
    match choice {
        EmbedderChoice::SignHash => (None, None),
        EmbedderChoice::Onnx { model, tokenizer } => (
            Some(model.display().to_string()),
            tokenizer
                .clone()
                .or_else(|| explicit_tokenizer.clone())
                .map(|p| p.display().to_string()),
        ),
        EmbedderChoice::CodeDaemon { model, .. } => {
            let tok = explicit_tokenizer
                .clone()
                .or_else(|| {
                    let beside = default_tokenizer_path(model.parent().unwrap_or(repo));
                    beside.is_file().then_some(beside)
                })
                .or_else(|| {
                    let default = default_tokenizer_path(repo);
                    default.is_file().then_some(default)
                });
            (
                Some(model.display().to_string()),
                tok.map(|p| p.display().to_string()),
            )
        }
    }
}

struct EngineBlastProvider<'a> {
    repo: &'a Path,
    backend: &'a MemoryBackend,
    graph_digest: Option<String>,
}

impl BlastSummaryProvider for EngineBlastProvider<'_> {
    fn summarize(
        &self,
        anchor_id: Uuid,
    ) -> rbuilder_error::Result<Option<crate::analysis::SemanticBlastSummary>> {
        let result = if let Some(digest) = self.graph_digest.as_deref() {
            if let Some(engine) = try_load_engine(self.repo, digest)? {
                engine.analyze(anchor_id)?
            } else {
                BlastRadiusEngine::build(self.backend)?.analyze(anchor_id)?
            }
        } else {
            BlastRadiusEngine::build(self.backend)?.analyze(anchor_id)?
        };

        let node = self
            .backend
            .get_node(anchor_id)?
            .ok_or_else(|| rbuilder_error::Error::NodeNotFound(anchor_id.to_string()))?;

        Ok(Some(blast_summary_from_result(
            &crate::analysis::SemanticEntry {
                node_id: anchor_id,
                name: node.name,
                qualified_name: node.qualified_name,
                file_path: node.file_path,
                code_hash: node.code_hash,
            },
            result.direct_caller_ids.len(),
            result.impact_zone_ids.len(),
            result.score,
        )))
    }
}

fn expand_gql_neighbors(
    backend: &MemoryBackend,
    hits: &[crate::analysis::SemanticHit],
    depth: usize,
    anchor_limit: usize,
) -> Result<Vec<crate::analysis::SemanticExpandedNode>> {
    use crate::analysis::SemanticExpandedNode;
    use crate::gql::execute;

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for hit in hits.iter().take(anchor_limit) {
        let name = hit.entry.name.replace('\'', "''");
        let query = format!(
            "MATCH (a:Function)-[:CALLS*1..{depth}]->(b:Function) WHERE a.name = '{name}' RETURN b LIMIT 20"
        );
        let result = execute(backend, &query)?;
        for row in result.rows {
            for node in row.values() {
                if !seen.insert(node.id) {
                    continue;
                }
                out.push(SemanticExpandedNode {
                    node_id: node.id.to_string(),
                    name: node.name.clone(),
                    qualified_name: node.qualified_name.clone(),
                    file_path: node.file_path.clone(),
                    relation: "gql_calls".into(),
                    anchor_node_id: Some(hit.entry.node_id.to_string()),
                });
            }
        }
    }
    Ok(out)
}

fn print_expansion_text(exp: &SemanticExpansion) {
    if let Some(neighbors) = &exp.neighbors {
        if !neighbors.is_empty() {
            println!("\nNeighbors:");
            for node in neighbors.iter().take(10) {
                println!("  {} [{}]", node.name, node.relation);
            }
        }
    }
    if let Some(blast) = &exp.blast {
        if !blast.is_empty() {
            println!("\nBlast radius:");
            for summary in blast {
                println!(
                    "  {}  callers={} impact={} score={:.1}",
                    summary.anchor_name,
                    summary.direct_callers,
                    summary.impact_zone,
                    summary.score
                );
            }
        }
    }
    if let Some(gql) = &exp.gql {
        if !gql.is_empty() {
            println!("\nGQL expansion:");
            for node in gql.iter().take(10) {
                println!("  {} [{}]", node.name, node.relation);
            }
        }
    }
}

fn semantic_index_path(ctx: &CliContext) -> PathBuf {
    SemanticIndex::default_path(&ctx.repo)
}
