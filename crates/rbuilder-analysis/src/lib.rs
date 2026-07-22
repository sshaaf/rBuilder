//! Graph analysis algorithms for rBuilder

#![warn(missing_docs)]

pub mod alias;
pub mod ast_skeleton;
pub mod blast_engine_snapshot;
pub mod blast_radius;
pub mod blast_radius_scc;
pub mod blast_slice_handoff;
pub mod callgraph;
pub mod centrality;
pub mod centrality_approx;
pub mod cfg;
pub mod cfg_builder;
pub mod cfg_pdg_archive;
pub mod cold_metadata;
pub mod community;
pub mod community_label;
pub mod community_query;
pub mod complexity;
pub mod cpg;
pub mod cpg_export;
pub mod dataflow;
pub mod def_use;
pub mod dependency;
pub mod dominance;
pub mod field_write;
pub mod field_write_locals;
pub mod flow_cache;
pub mod graph_utils;
pub mod interprocedural_cfg;
pub mod interprocedural_slicing;
pub mod language_profile;
pub mod macro_call_index;
pub mod macro_call_lookup;
pub mod migration;
pub mod node_lookup;
pub mod pdg;
pub mod policy;
pub mod results;
pub mod semantic_code_daemon;
pub mod semantic_diffuse;
#[cfg(feature = "semantic-onnx")]
pub mod semantic_embedded;
pub mod semantic_embedder;
pub mod semantic_extract;
pub mod semantic_fusion;
pub mod semantic_hybrid;
#[cfg(feature = "semantic-onnx")]
pub mod semantic_onnx;
pub mod semantic_onnx_tokenizer;
pub mod semantic_search;
pub mod semantic_vocab;
pub mod slicing;
pub mod storage;
pub mod structural_topology;
pub mod taint;
pub mod type_inference;

pub use alias::may_alias_names;
pub use ast_skeleton::{
    build_function_skeleton, AstSkeletonArchive, AstSkeletonKind, AstSkeletonNode,
    AstSkeletonRecord, AST_SKELETON_ARCHIVE_FILE, AST_SKELETON_VERSION,
};
pub use blast_engine_snapshot::{try_load_engine, BlastEngineSnapshot, BLAST_SNAPSHOT_FILE};
pub use blast_radius::{
    resolve_unique_symbol, BlastRadiusAnalyzer, BlastRadiusReport, DataFlowImpact,
};
pub use blast_radius_scc::{
    impact_score_from_counts, BlastRadiusEngine, BlastRadiusResult, EngineStats, SccNode,
};
pub use blast_slice_handoff::{
    criterion_for_parameter, filter_handoff_seeds_by_index, load_source_files,
    resolve_handoff_seeds, resolve_handoff_seeds_for_indices, trace_blast_to_slices,
    trace_blast_to_slices_with_blast, BlastSliceTrace, SliceHandoffSeed,
};
pub use callgraph::CallGraph;
pub use centrality::{
    adaptive_pagerank_config, default_behavioral_edges, degree_centrality, BetweennessCentrality,
    CentralityAnalyzer, CentralityReport, CentralityRunSummary, CentralityScore, CentralityScores,
    DegreeCentrality, FastPageRank, FlatGraphIndex, HarmonicCentrality, PageRankStats,
    LARGE_GRAPH_PAGERANK_ITERATIONS, LARGE_GRAPH_PAGERANK_NODE_LIMIT,
    LARGE_GRAPH_PAGERANK_TOLERANCE, PAGERANK_TOLERANCE, STRUCTURAL_EDGE_TYPES,
};
pub use centrality_approx::{
    BetweennessMode, CentralityApproxStats, HarmonicMode, HyperBallHarmonic, SampledBetweenness,
    DEFAULT_EXACT_CENTRALITY_LIMIT, DEFAULT_HYPERBALL_ROUNDS, DEFAULT_SAMPLE_PIVOTS,
    HYPERBALL_EXACT_THRESHOLD, HYPERLOGLOG_PRECISION, LARGE_GRAPH_HYPERBALL_NODE_LIMIT,
    LARGE_GRAPH_HYPERBALL_ROUNDS,
};
pub use cfg::{
    BasicBlock, BlockId, CfgEdge, CfgEdgeType, ControlFlowGraph, Statement, StatementKind,
};
pub use cfg_builder::{
    build_cfg_for_function, build_cfg_for_function_in_tree, index_function_locations,
    FunctionLocation, ParsedSourceFile,
};
pub use cfg_pdg_archive::{CfgPdgArchive, CfgPdgRecord, CFG_PDG_ARCHIVE_FILE};
pub use cold_metadata::ColdMetadataDb;
pub use community::{
    default_community_edge_types, detect_communities, Community, CommunityDetector,
    CommunityResult, DashboardCommunity, HubStripPolicy, TieBreakStrategy, DEFAULT_HUB_SIGMA_K,
    DEFAULT_MAX_FROZEN_FRACTION, DEFAULT_MIN_NODES_FOR_HUB_STRIP,
};
pub use community_label::{
    dedupe_community_labels, fill_community_labels, fill_community_labels_from_nodes,
    infer_community_label, CommunityLabelHints,
};
pub use community_query::{
    is_virtual_community, CommunityInfo, CommunityQueryContext, VIRTUAL_COMMUNITY_PROP,
    VIRTUAL_COMMUNITY_VALUE,
};
pub use complexity::{classify_complexity, ComplexityAnalyzer, ComplexityLevel, ComplexityReport};
pub use cpg::{
    archive_path, cpg_calls, cpg_flows, cpg_function, cpg_mutations, cpg_status, CpgCallEdge,
    CpgCallsInfo, CpgFlowStep, CpgFlowsArgs, CpgFlowsResult, CpgFunctionInfo, CpgMutationHit,
    CpgMutationsResult, CpgStatus,
};
pub use cpg_export::{export_cpg, CpgExportFormat, CpgExportScope};
pub use dataflow::{compute_reaching_definitions, Definition, ReachingDefs};
pub use def_use::{extract_def_use, extract_used_variables};
pub use dependency::{CircularDependency, DependencyAnalyzer, ImpactResult};
pub use dominance::{verify_idom_acyclic, DominatorTree};
pub use field_write::{
    build_and_save_field_write_index, FieldWrite, FieldWriteIndex, FieldWriteKind, MutationQuery,
    FIELD_WRITE_INDEX_FILE,
};
pub use flow_cache::{CachedAnalysis, CfgPdgCache, FlowCache, NodePdgCache};
pub use graph_utils::{
    edge_type_set, filter_impact_by_caller_depth, PetGraphView, TraversalConfig,
    DEFAULT_TRAVERSAL_DEPTH,
};
pub use interprocedural_cfg::{
    InterproceduralCFG, InterproceduralCfgAccess, InterproceduralCfgView,
};
pub use interprocedural_slicing::{InterproceduralSlice, InterproceduralSlicer};
pub use language_profile::{
    canonical_language_id, cfg_language_id_from_path, cfg_language_ids, cfg_language_list,
    function_kinds_for, language_id_from_path, parse_source, profile_for_language,
    taint_enabled_for, LanguageAnalysisProfile,
};
pub use macro_call_index::{GraphFingerprint, MacroCallIndex, MacroCallIndexEntry, SymbolContext};
pub use macro_call_lookup::{
    candidates_from_backend, candidates_from_snapshot, canonical_fqn_from_node,
    canonical_fqn_from_qualified_name, class_name_from_node, inferred_target_metadata,
    language_from_node, parse_fqn_symbol, resolve_symbol_uuid, try_parse_symbol_uuid,
    MacroCallLookupDb, MacroCallLookupRow, MacroIndexEntry, ParsedSymbol,
};
pub use migration::{
    build_migration_graph, compute_migration_plan, MigrationCommunityEdge, MigrationCommunityNode,
    MigrationGraphPayload, MigrationOrderMode, MigrationPlanPayload, MigrationPlanStep,
    MigrationWeights, MIGRATION_GRAPH_SCHEMA_VERSION, MIGRATION_PLAN_SCHEMA_VERSION,
};
pub use node_lookup::NodeLookup;
pub use pdg::{
    ControlDependency, DataDepType, DataDependency, PdgBuildOptions, PdgNode, PdgNodeId,
    ProgramDependenceGraph,
};
pub use policy::{check_policies, evaluate_policies, DomainId, PolicyRegistry, PolicyViolation};
pub use results::{
    AnalysisResults, BlastRadiusMetrics, BlastRadiusTable, CentralityMetrics, CentralityTable,
    CommunityTable, ComplexityTable, StructuralSketchTable,
};
pub use semantic_code_daemon::{
    default_model_dir, default_model_path, default_tokenizer_path, validate_mrl_dimensions,
    CODE_DAEMON_MAX_SEQ_LEN, CODE_DAEMON_MODEL_ID, CODE_DAEMON_MRL_DIMS, CODE_DAEMON_NATIVE_DIMS,
    CODE_DAEMON_ONNX_FILE, CODE_DAEMON_TOKENIZER_FILE,
};
#[cfg(feature = "semantic-onnx")]
pub use semantic_code_daemon::{load_code_daemon_embedder, load_embedded_code_daemon_embedder};
pub use semantic_diffuse::{diffuse_call_topology, DiffuseConfig, DiffuseNeighborMode};
pub use semantic_embedder::{
    embedder_for_index, resolve_embedder, EmbedderChoice, OnnxReloadOptions, SemanticEmbedder,
    SignHashEmbedder,
};
pub use semantic_extract::{
    extract_body_tokens_for_node, extract_body_tokens_from_slice, resolve_source_path,
    FunctionTokenSketch, MIN_TOKEN_LEN,
};
pub use semantic_fusion::{
    entry_metadata_tokens, fuse_candidates, hamming_similarity, keyword_and_matches,
    name_overlap_score, query_index_with_fusion, query_keywords, FusionCandidate,
    SemanticFusionConfig, DEFAULT_CANDIDATE_POOL,
};
pub use semantic_hybrid::{
    blast_summary_from_result, expand_call_neighbors, expand_semantic_hits, BlastSummaryProvider,
    SemanticBlastSummary, SemanticExpandConfig, SemanticExpandMode, SemanticExpandedNode,
    SemanticExpansion,
};
pub use semantic_search::{
    build_from_backend, build_index, embed_text_for_function, embed_text_for_node,
    hamming_distance, hamming_top_k, quantize_binary, query_communities, query_index,
    query_index_with_embedder, sign_hash_embed, CommunitySemanticHit, SemanticBuildOptions,
    SemanticBuildStats, SemanticEntry, SemanticHit, SemanticIndex, DEFAULT_EMBEDDING_DIMENSIONS,
    SEMANTIC_INDEX_FILE, SEMANTIC_INDEX_SCHEMA_VERSION, SIGN_HASH_MODEL_ID,
};
pub use semantic_vocab::{
    TokenSpaceAccumulator, VocabAccumulateEmbedder, VOCAB_ACCUMULATE_MODEL_ID,
    VOCAB_NATIVE_DIMENSIONS,
};
pub use slicing::{
    compute_slice, compute_slice_with_options, BackwardSlicer, CodeSlice, ForwardSlicer,
    SliceCriterion, SliceDirection, SliceOptions,
};
pub use storage::{AnalysisIndexEntry, AnalysisStorage, FunctionAnalysis, FunctionIdSyncEntry};
pub use structural_topology::StructuralTopology;
pub use taint::{Sanitizer, TaintAnalyzer, TaintFlow, TaintSink, TaintSource};
pub use type_inference::{confidence_for, InferredType, TypeInferenceEngine, VariableType};
