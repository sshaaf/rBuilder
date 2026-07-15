//! Parallel CFG/PDG/taint analysis for discover `--cfg` / `--all`.

use crate::analysis::{
    build_cfg_for_function, cfg_language_id_from_path, AnalysisIndexEntry, AnalysisStorage,
    CfgPdgRecord, ControlFlowGraph, DominatorTree, FunctionAnalysis, FunctionIdSyncEntry,
    ParsedSourceFile, ProgramDependenceGraph, TaintAnalyzer,
};
use crate::analysis::storage::stable_function_key;
use rbuilder_graph::code_index::hash_code;
use rbuilder_graph::schema::Node;
use rbuilder_pipeline::with_pool;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::info;

/// Options for the CFG analysis batch.
#[derive(Debug, Clone, Default)]
pub struct CfgAnalysisOptions {
    /// Emit per-stage and tail-latency profile lines.
    pub verbose: bool,
    /// Optional Rayon thread count (`None` = global pool default).
    pub thread_count: Option<usize>,
}

/// Wall-clock totals for CFG sub-stages (sum of per-function work).
#[derive(Debug, Clone, Copy, Default)]
pub struct CfgStageProfile {
    pub build_cfg_secs: f64,
    pub dominator_secs: f64,
    pub pdg_secs: f64,
    pub taint_secs: f64,
}

/// Aggregated CFG pass results for discover reporting and archive export.
#[derive(Debug, Default)]
pub struct CfgAnalysisBatchResult {
    pub success_count: usize,
    pub error_count: usize,
    pub total_flows: usize,
    pub vulnerable_flows: usize,
    pub cache_hits: usize,
    pub recomputed: usize,
    pub skipped_unchanged: usize,
    pub orphans_removed: usize,
    pub archive_records: Vec<CfgPdgRecord>,
    pub archive_unchanged: bool,
    pub stage_profile: Option<CfgStageProfile>,
}

#[derive(Default)]
struct CfgStageTimings {
    build_cfg_ns: AtomicU64,
    dominator_ns: AtomicU64,
    pdg_ns: AtomicU64,
    taint_ns: AtomicU64,
    functions: AtomicU64,
}

struct CfgFunctionTiming {
    file_path: String,
    function_name: String,
    blocks: usize,
    total_ns: u64,
}

struct CfgFunctionWork {
    analysis: Option<FunctionAnalysis>,
    archive_record: Option<CfgPdgRecord>,
    flow_count: usize,
    vulnerable_count: usize,
    from_cache: bool,
    skip_persist: bool,
}

struct CfgIncrementalCache {
    index: HashMap<String, AnalysisIndexEntry>,
}

struct FunctionWorkItem {
    func_idx: usize,
    file_path: String,
    language: String,
}

struct FileSourceCache {
    sources: HashMap<String, Arc<String>>,
}

struct CfgWorkContext<'a> {
    functions: &'a [Node],
    sources: &'a FileSourceCache,
    cache: &'a CfgIncrementalCache,
    stage: Option<&'a CfgStageTimings>,
    timings: Option<&'a Mutex<Vec<CfgFunctionTiming>>>,
}

/// Analyze all repository functions in parallel with incremental reuse and bincode persistence.
pub fn run_cfg_analysis_batch(
    functions: &[Node],
    storage: &AnalysisStorage,
    repo_root: &Path,
    options: CfgAnalysisOptions,
) -> CfgAnalysisBatchResult {
    let cache = load_incremental_cache(storage, repo_root);

    if let Some(shortcut) = try_full_incremental_shortcut(functions, &cache) {
        let active_keys = active_stable_keys(functions, None);
        let _ = sync_storage_function_ids(storage, functions);
        let mut result = shortcut;
        result.orphans_removed = storage
            .purge_stale_by_stable_keys(&active_keys)
            .unwrap_or(0);
        return result;
    }

    let sources = preload_file_sources(functions, options.thread_count);
    let work_items = flatten_work_items(functions);
    let stage = options.verbose.then(CfgStageTimings::default);
    let stage_ref = stage.as_ref();
    let timing_log = options.verbose.then(|| Mutex::new(Vec::<CfgFunctionTiming>::new()));
    let timing_ref = timing_log.as_ref();

    let ctx = CfgWorkContext {
        functions,
        sources: &sources,
        cache: &cache,
        stage: stage_ref,
        timings: timing_ref,
    };

    let flat: Vec<Option<CfgFunctionWork>> = with_pool(options.thread_count, || {
        work_items
            .par_iter()
            .map_init(
                || HashMap::<String, ParsedSourceFile>::new(),
                |parse_cache, item| process_function_work_item(parse_cache, item, &ctx),
            )
            .collect()
    });

    let stage_profile = stage_ref.map(|stage| {
        let analyzed = flat.iter().filter_map(|w| w.as_ref()).count();
        emit_stage_profile(stage, analyzed)
    });
    if let (Some(log), true) = (timing_log, options.verbose) {
        emit_tail_profile(&log);
    }

    let mut saves: Vec<FunctionAnalysis> = Vec::new();
    let mut result = CfgAnalysisBatchResult::default();
    for work in flat {
        match work {
            None => result.error_count += 1,
            Some(w) => {
                if w.from_cache {
                    result.cache_hits += 1;
                } else {
                    result.recomputed += 1;
                }
                if w.skip_persist {
                    result.skipped_unchanged += 1;
                }
                result.success_count += 1;
                result.total_flows += w.flow_count;
                result.vulnerable_flows += w.vulnerable_count;
                if let Some(record) = w.archive_record {
                    result.archive_records.push(record);
                }
                if let Some(analysis) = w.analysis {
                    if !w.skip_persist {
                        saves.push(analysis);
                    }
                }
            }
        }
    }

    with_pool(options.thread_count, || {
        saves.par_iter().for_each(|analysis| {
            let _ = storage.save_function_no_index(analysis);
        });
    });
    let _ = storage.refresh_analysis_index_from_analyses(&saves);

    let active_keys = active_stable_keys(functions, Some(&sources));
    let _ = sync_storage_function_ids(storage, functions);
    result.orphans_removed = storage
        .purge_stale_by_stable_keys(&active_keys)
        .unwrap_or(0);
    result.archive_unchanged =
        result.skipped_unchanged == result.success_count && result.recomputed == 0;
    result.stage_profile = stage_profile;
    result
}

fn emit_stage_profile(stage: &CfgStageTimings, analyzed: usize) -> CfgStageProfile {
    let build_ns = stage.build_cfg_ns.load(Ordering::Relaxed);
    let dom_ns = stage.dominator_ns.load(Ordering::Relaxed);
    let pdg_ns = stage.pdg_ns.load(Ordering::Relaxed);
    let taint_ns = stage.taint_ns.load(Ordering::Relaxed);
    let profile = CfgStageProfile {
        build_cfg_secs: build_ns as f64 / 1_000_000_000.0,
        dominator_secs: dom_ns as f64 / 1_000_000_000.0,
        pdg_secs: pdg_ns as f64 / 1_000_000_000.0,
        taint_secs: taint_ns as f64 / 1_000_000_000.0,
    };

    let fns = stage.functions.load(Ordering::Relaxed).max(analyzed as u64);
    let denom = fns.max(1) as f64;
    for (stage_name, secs) in [
        ("cfg_build", profile.build_cfg_secs),
        ("cfg_dominator", profile.dominator_secs),
        ("cfg_pdg", profile.pdg_secs),
        ("cfg_taint", profile.taint_secs),
    ] {
        if secs > 0.0 {
            info!(
                target: "profile",
                stage = stage_name,
                secs,
                avg_ms_per_fn = secs * 1000.0 / denom,
                analyzed,
                "[profile] stage"
            );
        }
    }

    profile
}

fn emit_tail_profile(log: &Mutex<Vec<CfgFunctionTiming>>) {
    let Ok(mut entries) = log.lock() else {
        return;
    };
    if entries.is_empty() {
        return;
    }
    entries.sort_by(|a, b| b.total_ns.cmp(&a.total_ns));
    let p99_idx = ((entries.len() as f64 * 0.99).ceil() as usize).saturating_sub(1);
    let p99 = entries[p99_idx].total_ns as f64 / 1_000_000.0;
    info!(
        target: "profile",
        p99_ms = p99,
        "[profile] cfg function tail latency"
    );
    for entry in entries.iter().take(10) {
        info!(
            target: "profile",
            file = %entry.file_path,
            function = %entry.function_name,
            blocks = entry.blocks,
            total_ms = entry.total_ns as f64 / 1_000_000.0,
            "[profile] cfg slow function"
        );
    }
}

fn flatten_work_items(functions: &[Node]) -> Vec<FunctionWorkItem> {
    let mut items = Vec::new();
    for (idx, func) in functions.iter().enumerate() {
        let Some(file_path) = func.file_path.as_ref() else {
            continue;
        };
        let language = cfg_language_id_from_path(Path::new(file_path))
            .unwrap_or("")
            .to_string();
        if language.is_empty() {
            continue;
        }
        items.push(FunctionWorkItem {
            func_idx: idx,
            file_path: file_path.clone(),
            language,
        });
    }
    items
}

fn process_function_work_item(
    parse_cache: &mut HashMap<String, ParsedSourceFile>,
    item: &FunctionWorkItem,
    ctx: &CfgWorkContext<'_>,
) -> Option<CfgFunctionWork> {
    let func = &ctx.functions[item.func_idx];
    let source_arc = ctx.sources.sources.get(&item.file_path)?;
    let source = source_arc.as_str();
    analyze_function_in_file(
        func,
        &item.language,
        &item.file_path,
        source,
        parse_cache,
        ctx.cache,
        ctx.stage,
        ctx.timings,
    )
}

fn sync_storage_function_ids(storage: &AnalysisStorage, functions: &[Node]) -> usize {
    let refs: Vec<FunctionIdSyncEntry<'_>> = functions
        .iter()
        .filter_map(|func| {
            Some(FunctionIdSyncEntry {
                function_id: func.id,
                function_name: &func.name,
                file_path: func.file_path.as_ref()?,
                code_hash: func.code_hash.as_ref()?,
            })
        })
        .collect();
    storage.sync_index_function_ids(&refs).unwrap_or(0)
}

fn load_incremental_cache(storage: &AnalysisStorage, _repo_root: &Path) -> CfgIncrementalCache {
    let index = storage.load_analysis_index().unwrap_or_default();
    CfgIncrementalCache { index }
}

fn preload_file_sources(functions: &[Node], thread_count: Option<usize>) -> FileSourceCache {
    let paths: HashSet<String> = functions.iter().filter_map(|n| n.file_path.clone()).collect();
    let sources: HashMap<String, Arc<String>> = with_pool(thread_count, || {
        paths
            .par_iter()
            .filter_map(|path| {
                let content = std::fs::read_to_string(path).ok()?;
                Some((path.clone(), Arc::new(content)))
            })
            .collect()
    });
    FileSourceCache { sources }
}

fn active_stable_keys(
    functions: &[Node],
    sources: Option<&FileSourceCache>,
) -> HashSet<String> {
    let mut keys = HashSet::new();
    for func in functions {
        let Some(file_path) = func.file_path.as_ref() else {
            continue;
        };
        let hash = if let Some(code_hash) = func.code_hash.as_ref() {
            code_hash.clone()
        } else if let Some(cache) = sources {
            let Some(source) = cache.sources.get(file_path) else {
                continue;
            };
            resolve_code_hash(func, source)
        } else {
            continue;
        };
        keys.insert(stable_function_key(file_path, &func.name, &hash));
    }
    keys
}

fn resolve_code_hash(func_node: &Node, source: &str) -> String {
    func_node
        .code_hash
        .clone()
        .unwrap_or_else(|| hash_code(source))
}

fn analyze_function_in_file(
    func_node: &Node,
    language: &str,
    file_path: &str,
    source: &str,
    parse_cache: &mut HashMap<String, ParsedSourceFile>,
    cache: &CfgIncrementalCache,
    stage: Option<&CfgStageTimings>,
    timings: Option<&Mutex<Vec<CfgFunctionTiming>>>,
) -> Option<CfgFunctionWork> {
    let code_hash = resolve_code_hash(func_node, source);

    if func_node.code_hash.is_some() {
        if let Some(hit) = try_fast_cache_hit(func_node, file_path, &code_hash, cache) {
            return Some(hit);
        }
        if let Some(hit) = try_remap_cached(func_node, file_path, &code_hash, cache) {
            return Some(hit);
        }
    }

    compute_function_cfg(
        func_node,
        language,
        file_path,
        source,
        &code_hash,
        parse_cache,
        false,
        stage,
        timings,
    )
}

fn try_fast_cache_hit(
    func_node: &Node,
    file_path: &str,
    code_hash: &str,
    cache: &CfgIncrementalCache,
) -> Option<CfgFunctionWork> {
    let key = stable_function_key(file_path, &func_node.name, code_hash);
    let entry = cache.index.get(&key)?;
    if entry.code_hash != code_hash || entry.function_id != func_node.id {
        return None;
    }
    Some(CfgFunctionWork {
        analysis: None,
        archive_record: None,
        flow_count: entry.flow_count,
        vulnerable_count: entry.vulnerable_count,
        from_cache: true,
        skip_persist: true,
    })
}

fn try_remap_cached(
    func_node: &Node,
    file_path: &str,
    code_hash: &str,
    cache: &CfgIncrementalCache,
) -> Option<CfgFunctionWork> {
    let key = stable_function_key(file_path, &func_node.name, code_hash);
    let entry = cache.index.get(&key)?;
    if entry.code_hash != code_hash || entry.function_id == func_node.id {
        return None;
    }
    Some(CfgFunctionWork {
        analysis: None,
        archive_record: None,
        flow_count: entry.flow_count,
        vulnerable_count: entry.vulnerable_count,
        from_cache: true,
        skip_persist: true,
    })
}

fn compute_function_cfg(
    func_node: &Node,
    language: &str,
    file_path: &str,
    source: &str,
    code_hash: &str,
    parse_cache: &mut HashMap<String, ParsedSourceFile>,
    from_cache: bool,
    stage: Option<&CfgStageTimings>,
    timings: Option<&Mutex<Vec<CfgFunctionTiming>>>,
) -> Option<CfgFunctionWork> {
    let total_start = timings.is_some().then(Instant::now);

    let build_start = stage.map(|_| Instant::now());
    let bytes = source.as_bytes();
    let cfg_data = if let Some(parsed) = parse_cache.get(file_path) {
        parsed
            .build_cfg(language, bytes, &func_node.name)
            .ok()
    } else {
        None
    }
    .or_else(|| {
        ParsedSourceFile::parse(language, bytes)
            .ok()
            .and_then(|parsed| {
                let cfg = parsed.build_cfg(language, bytes, &func_node.name).ok()?;
                parse_cache.insert(file_path.to_string(), parsed);
                Some(cfg)
            })
    })
    .or_else(|| build_cfg_for_function(language, source, &func_node.name).ok())?;
    if let (Some(stage), Some(start)) = (stage, build_start) {
        stage
            .build_cfg_ns
            .fetch_add(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }

    let work = compute_from_cfg(
        func_node,
        file_path,
        source,
        code_hash,
        language,
        cfg_data,
        from_cache,
        stage,
    )?;

    if let (Some(start), Some(log)) = (total_start, timings) {
        if let Ok(mut entries) = log.lock() {
            entries.push(CfgFunctionTiming {
                file_path: file_path.to_string(),
                function_name: func_node.name.clone(),
                blocks: work
                    .analysis
                    .as_ref()
                    .and_then(|a| a.cfg.as_ref())
                    .map(|c| c.blocks.len())
                    .unwrap_or(0),
                total_ns: start.elapsed().as_nanos() as u64,
            });
        }
    }

    Some(work)
}

fn compute_from_cfg(
    func_node: &Node,
    file_path: &str,
    source: &str,
    code_hash: &str,
    language: &str,
    cfg_data: ControlFlowGraph,
    from_cache: bool,
    stage: Option<&CfgStageTimings>,
) -> Option<CfgFunctionWork> {
    if let Some(stage) = stage {
        stage.functions.fetch_add(1, Ordering::Relaxed);
    }

    let dom_start = stage.map(|_| Instant::now());
    let dom_data = DominatorTree::build(&cfg_data);
    if let (Some(stage), Some(start)) = (stage, dom_start) {
        stage
            .dominator_ns
            .fetch_add(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }

    let pdg_start = stage.map(|_| Instant::now());
    let pdg_data =
        ProgramDependenceGraph::build_with_dominator(&cfg_data, source.as_bytes(), &dom_data).ok();
    if let (Some(stage), Some(start)) = (stage, pdg_start) {
        stage
            .pdg_ns
            .fetch_add(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }

    let taint_start = stage.map(|_| Instant::now());
    let (taint_data, flow_count, vulnerable_count) = if let Some(ref pdg) = pdg_data {
        let mut analyzer = TaintAnalyzer::with_dominator(pdg, &cfg_data, dom_data);
        analyzer.detect_patterns(language);
        let flows = analyzer.analyze();
        let vulnerable = flows.iter().filter(|f| f.is_vulnerable()).count();
        let count = flows.len();
        let taint = if flows.is_empty() { None } else { Some(flows) };
        (taint, count, vulnerable)
    } else {
        (None, 0, 0)
    };
    if let (Some(stage), Some(start)) = (stage, taint_start) {
        stage
            .taint_ns
            .fetch_add(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }

    let analysis = FunctionAnalysis {
        function_id: func_node.id,
        function_name: func_node.name.clone(),
        file_path: file_path.to_string(),
        code_hash: Some(code_hash.to_string()),
        cfg: Some(cfg_data),
        pdg: pdg_data,
        dominance: None,
        taint: taint_data,
    };

    let archive_record = archive_record_from_analysis(func_node, file_path, code_hash, &analysis);

    Some(CfgFunctionWork {
        analysis: Some(analysis),
        archive_record,
        flow_count,
        vulnerable_count,
        from_cache,
        skip_persist: false,
    })
}

fn archive_record_from_analysis(
    func_node: &Node,
    file_path: &str,
    code_hash: &str,
    analysis: &FunctionAnalysis,
) -> Option<CfgPdgRecord> {
    match (&analysis.cfg, &analysis.pdg) {
        (Some(cfg), Some(pdg)) => Some(CfgPdgRecord {
            function_id: func_node.id,
            code_hash: code_hash.to_string(),
            function_name: func_node.name.clone(),
            file_path: Some(file_path.to_string()),
            cfg: cfg.clone(),
            pdg: pdg.clone(),
        }),
        _ => None,
    }
}

/// When every CFG-eligible function is already indexed with the same body hash, skip the batch.
fn try_full_incremental_shortcut(
    functions: &[Node],
    cache: &CfgIncrementalCache,
) -> Option<CfgAnalysisBatchResult> {
    let mut total_flows = 0usize;
    let mut vulnerable_flows = 0usize;
    let mut matched = 0usize;
    let mut eligible = 0usize;

    for func in functions {
        let Some(file_path) = func.file_path.as_ref() else {
            continue;
        };
        let Some(code_hash) = func.code_hash.as_ref() else {
            continue;
        };
        if cfg_language_id_from_path(Path::new(file_path)).is_none() {
            continue;
        }
        eligible += 1;
        let key = stable_function_key(file_path, &func.name, code_hash);
        let entry = cache.index.get(&key)?;
        if entry.code_hash != *code_hash {
            return None;
        }
        matched += 1;
        total_flows += entry.flow_count;
        vulnerable_flows += entry.vulnerable_count;
    }

    if eligible == 0 || matched != eligible {
        return None;
    }

    Some(CfgAnalysisBatchResult {
        success_count: matched,
        total_flows,
        vulnerable_flows,
        cache_hits: matched,
        skipped_unchanged: matched,
        archive_unchanged: true,
        ..CfgAnalysisBatchResult::default()
    })
}
