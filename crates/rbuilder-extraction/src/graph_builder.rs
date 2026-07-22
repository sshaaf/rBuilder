//! Maps extracted symbols and relations into graph nodes and edges.

use rbuilder_error::{Error, Result};
use rbuilder_graph::code_index::{hash_code, CodeIndex};
use rbuilder_graph::migration::graph_parameter_from_plugin;
use rbuilder_graph::schema::{Edge, EdgeType, Node, NodeType};
use rbuilder_graph::segmented_spill::{FinishedSpill, SegmentedSpill};
use rbuilder_graph::structural_sketch::build_token_bloom;
use rbuilder_plugin_api::{
    ComplexityMetrics, ConfigKey, Relation, RelationType, Symbol, SymbolType,
};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
struct LineSpan {
    start: usize,
    end: usize,
    id: Uuid,
}

/// Builds graph nodes and edges from extracted data.
#[derive(Default)]
pub struct GraphBuilder {
    symbol_index: HashMap<String, Uuid>,
    file_nodes: HashMap<String, Uuid>,
    config_key_nodes: HashMap<String, Uuid>,
    env_nodes: HashMap<String, Uuid>,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    /// Optional disk spill — when set, nodes/edges are not kept in `Vec`s.
    spill: Option<SegmentedSpill>,
    spill_error: Option<String>,
    spilled_nodes: usize,
    spilled_edges: usize,
    /// Line ranges for config-usage resolution when nodes are spilled.
    file_line_spans: HashMap<String, Vec<LineSpan>>,
    code_index: Option<CodeIndex>,
    // Resolution performance tracking
    resolution_stats: ResolutionStats,
    // Fast resolution indexes (built on demand)
    /// Qualified name → candidate UUIDs (may be ambiguous when FQNs collide).
    symbols_by_qualified: HashMap<String, Vec<Uuid>>,
    symbols_by_suffix: HashMap<String, Vec<Uuid>>,
    indexes_built: bool,
}

#[derive(Debug, Default)]
struct ResolutionStats {
    total_calls: usize,
    hashmap_hits: usize,
    qualified_hint_scans: usize,
    qualified_hint_hits: usize, // O(1) index hits
    type_hint_scans: usize,
    type_hint_hits: usize, // O(1) index hits
    fuzzy_scans: usize,
    fuzzy_hits: usize, // O(1) index hits
    total_time: std::time::Duration,
    line_lookups: usize,
    line_lookup_time: std::time::Duration,
}

impl GraphBuilder {
    /// Create an empty graph builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder that spills nodes/edges to `spill_dir` instead of retaining Vecs.
    pub fn with_spill(spill_dir: impl AsRef<Path>) -> Result<Self> {
        let mut builder = Self::new();
        builder.spill = Some(SegmentedSpill::create(spill_dir)?);
        Ok(builder)
    }

    /// Whether this builder is spilling to disk.
    pub fn is_spilling(&self) -> bool {
        self.spill.is_some()
    }

    /// Number of nodes built so far.
    pub fn node_count(&self) -> usize {
        if self.spill.is_some() {
            self.spilled_nodes
        } else {
            self.nodes.len()
        }
    }

    /// Number of edges built so far.
    pub fn edge_count(&self) -> usize {
        if self.spill.is_some() {
            self.spilled_edges
        } else {
            self.edges.len()
        }
    }

    fn record_line_span(&mut self, node: &Node) {
        let Some(file) = node.file_path.as_deref() else {
            return;
        };
        let Some(start) = node.start_line else {
            return;
        };
        let end = node.end_line.unwrap_or(start);
        self.file_line_spans
            .entry(file.to_string())
            .or_default()
            .push(LineSpan {
                start,
                end,
                id: node.id,
            });
    }

    fn index_symbol_resolution(&mut self, key: &str, node: &Node) {
        if let Some(qualified) = &node.qualified_name {
            let entry = self
                .symbols_by_qualified
                .entry(qualified.clone())
                .or_default();
            if !entry.contains(&node.id) {
                entry.push(node.id);
            }
        }
        let parts: Vec<&str> = key.split("::").collect();
        for i in 1..parts.len() {
            let suffix = parts[i..].join("::");
            let entry = self.symbols_by_suffix.entry(suffix).or_default();
            if !entry.contains(&node.id) {
                entry.push(node.id);
            }
        }
    }

    fn commit_node(&mut self, node: Node) {
        self.record_line_span(&node);
        if let Some(spill) = self.spill.as_mut() {
            if let Err(e) = spill.append_node(&node) {
                self.spill_error = Some(e.to_string());
            } else {
                self.spilled_nodes += 1;
            }
        } else {
            self.nodes.push(node);
        }
    }

    fn commit_edge(&mut self, edge: Edge) {
        if let Some(spill) = self.spill.as_mut() {
            if let Err(e) = spill.append_edge(&edge) {
                self.spill_error = Some(e.to_string());
            } else {
                self.spilled_edges += 1;
            }
        } else {
            self.edges.push(edge);
        }
    }

    /// Ensure a file node exists and return its ID.
    pub fn ensure_file_node(&mut self, path: &Path) -> Uuid {
        let file_path = path.to_string_lossy().to_string();
        if let Some(id) = self.file_nodes.get(&file_path) {
            return *id;
        }

        let node = Node::new(NodeType::File, file_path.clone()).with_file_path(file_path.clone());
        let id = node.id;
        self.file_nodes.insert(file_path, id);
        self.commit_node(node);
        id
    }

    /// Attach a code index for body hashing during symbol insertion.
    pub fn set_code_index(&mut self, index: CodeIndex) {
        self.code_index = Some(index);
    }

    /// Mutable access to the optional code index.
    pub fn code_index_mut(&mut self) -> Option<&mut CodeIndex> {
        self.code_index.as_mut()
    }

    /// Add a symbol node linked to its file.
    pub fn add_symbol(&mut self, symbol: &Symbol, file_id: Uuid) -> Uuid {
        self.add_symbol_with_body(symbol, file_id, None)
    }

    /// Add a symbol node and optionally hash its body for change detection.
    pub fn add_symbol_with_body(
        &mut self,
        symbol: &Symbol,
        file_id: Uuid,
        body: Option<&str>,
    ) -> Uuid {
        let key = symbol_key(
            &symbol.location.file,
            &symbol.name,
            symbol.qualified_name.as_deref(),
        );
        if let Some(id) = self.symbol_index.get(&key) {
            return *id;
        }

        let mut node = Node::new(
            symbol_type_to_node_type(symbol.symbol_type),
            symbol.name.clone(),
        )
        .with_file_path(symbol.location.file.clone())
        .with_location(symbol.location.start_line, symbol.location.end_line);

        if let Some(qn) = &symbol.qualified_name {
            node = node.with_qualified_name(qn.clone());
        }
        if let Some(sig) = &symbol.signature {
            node = node.with_signature(sig.clone());
        }
        if let Some(ret) = &symbol.return_type {
            node = node.with_return_type(ret.clone());
        }
        if !symbol.parameters.is_empty() {
            node = node.with_parameters(
                symbol
                    .parameters
                    .iter()
                    .cloned()
                    .map(graph_parameter_from_plugin)
                    .collect(),
            );
        }
        if let Some(body) = body {
            let code_hash = if let Some(index) = self.code_index.as_mut() {
                index.add_code(body, &symbol.location)
            } else {
                hash_code(body)
            };
            node = node.with_code_hash(code_hash);
        }

        if should_sketch_symbol(symbol.symbol_type) {
            let bloom = build_token_bloom(
                &symbol.name,
                symbol.qualified_name.as_deref(),
                symbol.signature.as_deref(),
                body,
            );
            node = node.with_token_bloom(bloom);
        }
        if !symbol.modifiers.is_empty() {
            node = node.with_property("modifiers".to_string(), symbol.modifiers.join(" "));
        }
        if let Some(doc) = &symbol.documentation {
            node = node.with_property("documentation".to_string(), doc.clone());
        }
        if let Some(obj) = symbol.metadata.as_object() {
            for (k, v) in obj {
                let prop_val = match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Bool(b) => Some(b.to_string()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    _ => None,
                };
                if let Some(s) = prop_val {
                    node = node.with_property(k.clone(), s);
                }
            }
        }

        let id = node.id;
        self.symbol_index.insert(key.clone(), id);
        self.index_symbol_resolution(&key, &node);
        self.commit_node(node);
        self.add_edge(id, file_id, EdgeType::DefinedIn);
        self.add_edge(file_id, id, EdgeType::Contains);
        self.add_field_members(id, symbol, file_id);
        id
    }

    /// Materialize `Symbol.fields` as Variable nodes contained by the owning type.
    ///
    /// Enables hybrid CPG member queries without waiting for per-language Variable
    /// symbols. Idempotent via `symbol_index` keys (`Owner.field` FQN).
    fn add_field_members(&mut self, owner_id: Uuid, symbol: &Symbol, file_id: Uuid) {
        if symbol.fields.is_empty() {
            return;
        }
        let owner_qn = symbol
            .qualified_name
            .clone()
            .unwrap_or_else(|| symbol.name.clone());
        for field in &symbol.fields {
            let field_qn = format!("{owner_qn}.{}", field.name);
            let key = symbol_key(
                &symbol.location.file,
                &field.name,
                Some(field_qn.as_str()),
            );
            if self.symbol_index.contains_key(&key) {
                continue;
            }
            let mut node = Node::new(NodeType::Variable, field.name.clone())
                .with_file_path(symbol.location.file.clone())
                .with_location(symbol.location.start_line, symbol.location.end_line)
                .with_qualified_name(field_qn.clone())
                .with_label("field".to_string())
                .with_property("member_of".to_string(), symbol.name.clone())
                .with_property("owner_qualified_name".to_string(), owner_qn.clone());
            if let Some(ty) = &field.field_type {
                node = node.with_property("field_type".to_string(), ty.clone());
            }
            if let Some(vis) = &field.visibility {
                node = node.with_property("visibility".to_string(), vis.clone());
            }
            let field_id = node.id;
            self.symbol_index.insert(key.clone(), field_id);
            self.index_symbol_resolution(&key, &node);
            self.commit_node(node);
            self.add_edge(field_id, file_id, EdgeType::DefinedIn);
            self.add_edge(file_id, field_id, EdgeType::Contains);
            self.add_edge(owner_id, field_id, EdgeType::Contains);
        }
    }

    /// Attach complexity metrics to an existing symbol node.
    ///
    /// Only supported in in-memory mode (not when spilling).
    pub fn add_complexity(&mut self, symbol: &Symbol, metrics: &ComplexityMetrics) {
        if self.spill.is_some() {
            return;
        }
        let key = symbol_key(
            &symbol.location.file,
            &symbol.name,
            symbol.qualified_name.as_deref(),
        );
        if let Some(id) = self.symbol_index.get(&key) {
            if let Some(node) = self.nodes.iter_mut().find(|n| n.id == *id) {
                node.properties
                    .insert("cyclomatic".to_string(), metrics.cyclomatic.to_string());
                node.properties
                    .insert("cognitive".to_string(), metrics.cognitive.to_string());
                node.properties
                    .insert("loc".to_string(), metrics.loc.to_string());
                node.properties.insert(
                    "nesting_depth".to_string(),
                    metrics.nesting_depth.to_string(),
                );
            }
        }
    }

    /// Build reverse indexes for fast symbol resolution.
    ///
    /// Call this after all symbols are added but before processing relations.
    /// Indexes are maintained incrementally at insert; this finalizes the flag.
    pub fn build_resolution_indexes(&mut self) {
        use tracing::info;

        if self.indexes_built {
            return;
        }

        // Suffix / qualified indexes are filled in `index_symbol_resolution` at insert.
        // Rebuild suffix from symbol_index only if somehow empty (legacy / partial path).
        if self.symbols_by_suffix.is_empty() && !self.symbol_index.is_empty() {
            for key in self.symbol_index.keys() {
                let parts: Vec<&str> = key.split("::").collect();
                for i in 1..parts.len() {
                    let suffix = parts[i..].join("::");
                    if let Some(uuid) = self.symbol_index.get(key) {
                        let entry = self.symbols_by_suffix.entry(suffix).or_default();
                        if !entry.contains(uuid) {
                            entry.push(*uuid);
                        }
                    }
                }
            }
        }

        self.indexes_built = true;

        info!(
            symbol_count = self.symbol_index.len(),
            qualified_count = self.symbols_by_qualified.len(),
            suffix_count = self.symbols_by_suffix.len(),
            "built resolution indexes"
        );
    }

    /// Add a configuration key node linked to its file.
    pub fn add_config_key(&mut self, key: &ConfigKey, file_id: Uuid) -> Uuid {
        let lookup = format!("{}::{}", key.location.file, key.key_path);
        if let Some(id) = self.config_key_nodes.get(&lookup) {
            return *id;
        }

        let node = Node::new(NodeType::ConfigKey, key.key_path.clone())
            .with_file_path(key.location.file.clone())
            .with_property("value".to_string(), key.value.clone())
            .with_property("value_type".to_string(), format!("{:?}", key.value_type));

        let id = node.id;
        self.config_key_nodes.insert(lookup, id);
        self.commit_node(node);
        self.add_edge(id, file_id, EdgeType::DefinedIn);
        self.add_edge(file_id, id, EdgeType::Contains);
        id
    }

    /// Add a relation between symbols if both endpoints exist.
    pub fn add_relation(&mut self, relation: &Relation) -> Result<()> {
        let from_id =
            self.resolve_symbol_tracked(&relation.from, &relation.location.file, None, None);

        // Use hints for cross-file resolution (best-effort)
        // Hints are language plugin's best guess at the qualified name based on local context
        let to_id = self.resolve_symbol_tracked(
            &relation.to,
            &relation.location.file,
            relation.to_qualified_hint.as_deref(),
            relation.to_type_hint.as_deref(),
        );

        if let (Some(from), Some(to)) = (from_id, to_id) {
            let edge_type = relation_type_to_edge_type(relation.relation_type);
            let mut edge = Edge::new(from, to, edge_type);
            if relation.relation_type == RelationType::Calls {
                edge = edge.with_property(
                    "call_site_line".to_string(),
                    relation.location.start_line.to_string(),
                );
            }
            self.commit_edge(edge);
        }
        Ok(())
    }

    /// Link code to a configuration key or environment variable usage.
    pub fn link_config_usage(
        &mut self,
        file_path: &str,
        line: usize,
        key: &str,
        usage_type: ConfigUsageKind,
    ) {
        let file_id = self.file_nodes.get(file_path).copied();
        let code_node = self
            .find_symbol_at_line_tracked(file_path, line)
            .or(file_id);

        let Some(from_id) = code_node else {
            return;
        };

        let target_id = match usage_type {
            ConfigUsageKind::EnvVar => self.ensure_env_node(key),
            ConfigUsageKind::ConfigKey => self.ensure_config_key_node(key, file_path),
        };

        self.add_edge(from_id, target_id, EdgeType::UsesConfig);
    }

    fn ensure_env_node(&mut self, key: &str) -> Uuid {
        if let Some(id) = self.env_nodes.get(key) {
            return *id;
        }

        let node = Node::new(NodeType::Variable, key.to_string())
            .with_label("env".to_string())
            .with_property("env_var".to_string(), key.to_string());

        let id = node.id;
        self.env_nodes.insert(key.to_string(), id);
        self.commit_node(node);
        id
    }

    fn ensure_config_key_node(&mut self, key: &str, file_path: &str) -> Uuid {
        let lookup = format!("{file_path}::{key}");
        if let Some(id) = self.config_key_nodes.get(&lookup) {
            return *id;
        }

        let node =
            Node::new(NodeType::ConfigKey, key.to_string()).with_file_path(file_path.to_string());
        let id = node.id;
        self.config_key_nodes.insert(lookup, id);
        self.commit_node(node);
        id
    }

    fn find_symbol_at_line_tracked(&mut self, file_path: &str, line: usize) -> Option<Uuid> {
        use std::time::Instant;

        let start = Instant::now();
        self.resolution_stats.line_lookups += 1;

        let result = if self.spill.is_some() {
            self.file_line_spans
                .get(file_path)
                .and_then(|spans| {
                    spans
                        .iter()
                        .filter(|s| s.start <= line && s.end >= line)
                        .max_by_key(|s| s.start)
                        .map(|s| s.id)
                })
        } else {
            self.nodes
                .iter()
                .filter(|n| n.file_path.as_deref() == Some(file_path))
                .filter(|n| {
                    n.start_line
                        .map(|start| start <= line && n.end_line.unwrap_or(start) >= line)
                        .unwrap_or(false)
                })
                .max_by_key(|n| n.start_line.unwrap_or(0))
                .map(|n| n.id)
        };

        self.resolution_stats.line_lookup_time += start.elapsed();
        result
    }

    #[allow(dead_code)]
    fn find_symbol_at_line(&self, file_path: &str, line: usize) -> Option<Uuid> {
        if self.spill.is_some() {
            return self.file_line_spans.get(file_path).and_then(|spans| {
                spans
                    .iter()
                    .filter(|s| s.start <= line && s.end >= line)
                    .max_by_key(|s| s.start)
                    .map(|s| s.id)
            });
        }
        self.nodes
            .iter()
            .filter(|n| n.file_path.as_deref() == Some(file_path))
            .filter(|n| {
                n.start_line
                    .map(|start| start <= line && n.end_line.unwrap_or(start) >= line)
                    .unwrap_or(false)
            })
            .max_by_key(|n| n.start_line.unwrap_or(0))
            .map(|n| n.id)
    }

    /// Resolve a symbol name to its UUID with performance tracking.
    ///
    /// Ambiguous qualified-name or suffix matches return `None` (do not pick arbitrarily).
    fn resolve_symbol_tracked(
        &mut self,
        name: &str,
        file: &str,
        qualified_hint: Option<&str>,
        type_hint: Option<&str>,
    ) -> Option<Uuid> {
        use std::time::Instant;

        let start = Instant::now();
        self.resolution_stats.total_calls += 1;

        // 1. Try exact match in current file
        let qualified = format!("{file}::{name}");
        if let Some(id) = self.symbol_index.get(&qualified) {
            self.resolution_stats.hashmap_hits += 1;
            self.resolution_stats.total_time += start.elapsed();
            return Some(*id);
        }

        // 2. Try qualified hint direct lookup (O(1)); only if uniquely resolved
        if let Some(hint) = qualified_hint {
            self.resolution_stats.qualified_hint_scans += 1;

            if let Some(id) = self
                .symbols_by_qualified
                .get(hint)
                .and_then(|ids| unique_resolved(ids))
            {
                self.resolution_stats.qualified_hint_hits += 1;
                self.resolution_stats.total_time += start.elapsed();
                return Some(id);
            }

            if let Some(id) = self
                .symbols_by_suffix
                .get(hint)
                .and_then(|ids| unique_resolved(ids))
            {
                self.resolution_stats.qualified_hint_hits += 1;
                self.resolution_stats.total_time += start.elapsed();
                return Some(id);
            }
        }

        // 3. Try type hint + simple name (O(1)); only if uniquely resolved
        if let Some(type_name) = type_hint {
            self.resolution_stats.type_hint_scans += 1;
            let simple_name = name.split('.').next_back().unwrap_or(name);
            let type_qualified = format!("{type_name}.{simple_name}");

            if let Some(id) = self
                .symbols_by_suffix
                .get(&type_qualified)
                .and_then(|ids| unique_resolved(ids))
            {
                self.resolution_stats.type_hint_hits += 1;
                self.resolution_stats.total_time += start.elapsed();
                return Some(id);
            }
        }

        // 4. Fallback: suffix index — None when zero or multiple candidates
        self.resolution_stats.fuzzy_scans += 1;
        let result = self
            .symbols_by_suffix
            .get(name)
            .and_then(|ids| unique_resolved(ids));

        if result.is_some() {
            self.resolution_stats.fuzzy_hits += 1;
        }

        self.resolution_stats.total_time += start.elapsed();
        result
    }

    /// Resolve a symbol name to its UUID (internal use without tracking).
    ///
    /// Resolution strategy (in order):
    /// 1. Try exact match in current file: `{file}::{name}`
    /// 2. If qualified_hint provided, try unique match on qualified / suffix indexes
    /// 3. If type_hint provided, try unique suffix match for `{type}.{name}`
    /// 4. Fallback to unique fuzzy suffix match on `name`
    ///
    /// Ambiguous matches return `None`.
    #[allow(dead_code)]
    fn resolve_symbol(
        &self,
        name: &str,
        file: &str,
        qualified_hint: Option<&str>,
        type_hint: Option<&str>,
    ) -> Option<Uuid> {
        let qualified = format!("{file}::{name}");
        if let Some(id) = self.symbol_index.get(&qualified) {
            return Some(*id);
        }

        if let Some(hint) = qualified_hint {
            if let Some(id) = self
                .symbols_by_qualified
                .get(hint)
                .and_then(|ids| unique_resolved(ids))
            {
                return Some(id);
            }
            if let Some(id) = self
                .symbols_by_suffix
                .get(hint)
                .and_then(|ids| unique_resolved(ids))
            {
                return Some(id);
            }
        }

        if let Some(type_name) = type_hint {
            let simple_name = name.split('.').next_back().unwrap_or(name);
            let type_qualified = format!("{type_name}.{simple_name}");
            if let Some(id) = self
                .symbols_by_suffix
                .get(&type_qualified)
                .and_then(|ids| unique_resolved(ids))
            {
                return Some(id);
            }
        }

        self.symbols_by_suffix
            .get(name)
            .and_then(|ids| unique_resolved(ids))
    }

    /// Log resolution performance statistics.
    pub fn log_resolution_stats(&self) {
        use tracing::info;

        let stats = &self.resolution_stats;
        let avg_time_micros = if stats.total_calls > 0 {
            stats.total_time.as_micros() / stats.total_calls as u128
        } else {
            0
        };

        let avg_line_lookup_micros = if stats.line_lookups > 0 {
            stats.line_lookup_time.as_micros() / stats.line_lookups as u128
        } else {
            0
        };

        let scan_calls = stats.qualified_hint_scans + stats.type_hint_scans + stats.fuzzy_scans;
        let index_hits = stats.qualified_hint_hits + stats.type_hint_hits + stats.fuzzy_hits;
        let total_index_lookups = scan_calls;
        let index_hit_rate = if total_index_lookups > 0 {
            (index_hits as f64 / total_index_lookups as f64) * 100.0
        } else {
            0.0
        };

        info!(
            total_calls = stats.total_calls,
            hashmap_hits = stats.hashmap_hits,
            qualified_hint_scans = stats.qualified_hint_scans,
            qualified_hint_hits = stats.qualified_hint_hits,
            type_hint_scans = stats.type_hint_scans,
            type_hint_hits = stats.type_hint_hits,
            fuzzy_scans = stats.fuzzy_scans,
            fuzzy_hits = stats.fuzzy_hits,
            total_scan_calls = scan_calls,
            index_hits,
            index_hit_rate_percent = format!("{:.1}", index_hit_rate),
            total_time_secs = stats.total_time.as_secs_f64(),
            avg_time_micros,
            line_lookups = stats.line_lookups,
            line_lookup_time_secs = stats.line_lookup_time.as_secs_f64(),
            avg_line_lookup_micros,
            "symbol resolution statistics"
        );
    }

    fn add_edge(&mut self, from: Uuid, to: Uuid, edge_type: EdgeType) {
        self.commit_edge(Edge::new(from, to, edge_type));
    }

    /// Borrow built nodes (testing / inspection). Empty when spilling.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Consume the builder and return all nodes and edges.
    ///
    /// Panics if the builder was created with [`Self::with_spill`] — use
    /// [`Self::finish_spill`] instead.
    pub fn into_graph(self) -> (Vec<Node>, Vec<Edge>) {
        assert!(
            self.spill.is_none(),
            "into_graph called on spilling GraphBuilder; use finish_spill"
        );
        (self.nodes, self.edges)
    }

    /// Finish spill writers and return a [`FinishedSpill`] for columnar compile.
    pub fn finish_spill(mut self) -> Result<FinishedSpill> {
        if let Some(err) = self.spill_error.take() {
            return Err(Error::SerdeError(err));
        }
        let spill = self
            .spill
            .take()
            .ok_or_else(|| Error::SerdeError("finish_spill called without spill mode".into()))?;
        spill.finish()
    }
}

/// Kind of configuration reference detected in source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigUsageKind {
    /// Environment variable reference
    EnvVar,
    /// Configuration key reference
    ConfigKey,
}

fn symbol_key(file: &str, name: &str, qualified: Option<&str>) -> String {
    format!("{file}::{}", qualified.unwrap_or(name))
}

/// Return the sole candidate UUID, or `None` when zero or multiple distinct IDs.
fn unique_resolved(ids: &[Uuid]) -> Option<Uuid> {
    match ids {
        [id] => Some(*id),
        [first, rest @ ..] if rest.iter().all(|id| id == first) => Some(*first),
        _ => None,
    }
}

fn should_sketch_symbol(symbol_type: SymbolType) -> bool {
    matches!(symbol_type, SymbolType::Function)
}

fn symbol_type_to_node_type(symbol_type: SymbolType) -> NodeType {
    match symbol_type {
        SymbolType::Function => NodeType::Function,
        SymbolType::Class => NodeType::Class,
        SymbolType::Struct => NodeType::Struct,
        SymbolType::Enum => NodeType::Enum,
        SymbolType::Interface => NodeType::Interface,
        SymbolType::Module => NodeType::Module,
        SymbolType::Variable => NodeType::Variable,
        SymbolType::TypeAlias => NodeType::TypeAlias,
        SymbolType::Macro => NodeType::Macro,
        SymbolType::Import => NodeType::Import,
        SymbolType::Table => NodeType::Table,
        SymbolType::Dependency => NodeType::Dependency,
        SymbolType::Job => NodeType::Job,
        SymbolType::BuildStep => NodeType::BuildStep,
        SymbolType::AnsiblePlaybook => NodeType::AnsiblePlaybook,
        SymbolType::AnsiblePlay => NodeType::AnsiblePlay,
        SymbolType::AnsibleTask => NodeType::AnsibleTask,
        SymbolType::AnsibleRole => NodeType::AnsibleRole,
        SymbolType::AnsibleHandler => NodeType::AnsibleHandler,
        SymbolType::AnsibleVariable => NodeType::AnsibleVariable,
        SymbolType::AnsibleTemplate => NodeType::AnsibleTemplate,
        SymbolType::ChefCookbook => NodeType::ChefCookbook,
        SymbolType::ChefRecipe => NodeType::ChefRecipe,
        SymbolType::ChefResource => NodeType::ChefResource,
        SymbolType::ChefAttribute => NodeType::ChefAttribute,
        SymbolType::ChefTemplate => NodeType::ChefTemplate,
        SymbolType::ChefCustomResource => NodeType::ChefCustomResource,
        SymbolType::PuppetModule => NodeType::PuppetModule,
        SymbolType::PuppetClass => NodeType::PuppetClass,
        SymbolType::PuppetDefinedType => NodeType::PuppetDefinedType,
        SymbolType::PuppetResource => NodeType::PuppetResource,
        SymbolType::PuppetVariable => NodeType::PuppetVariable,
        SymbolType::PuppetFact => NodeType::PuppetFact,
    }
}

fn relation_type_to_edge_type(relation_type: RelationType) -> EdgeType {
    match relation_type {
        RelationType::Calls => EdgeType::Calls,
        RelationType::Uses => EdgeType::Uses,
        RelationType::Implements => EdgeType::Implements,
        RelationType::Extends => EdgeType::Extends,
        RelationType::Defines => EdgeType::Contains,
        RelationType::References => EdgeType::References,
        RelationType::Instantiates => EdgeType::Instantiates,
        RelationType::Modifies => EdgeType::Modifies,
        RelationType::DependsOn => EdgeType::DependsOn,
        RelationType::IncludesRole => EdgeType::IncludesRole,
        RelationType::DependsOnRole => EdgeType::DependsOnRole,
        RelationType::ExecutesTask => EdgeType::ExecutesTask,
        RelationType::NotifiesHandler => EdgeType::NotifiesHandler,
        RelationType::IncludesPlaybook => EdgeType::IncludesPlaybook,
        RelationType::UsesVariable => EdgeType::Uses,
        RelationType::RendersTemplate => EdgeType::RendersTemplate,
        RelationType::DependsOnCookbook => EdgeType::DependsOnCookbook,
        RelationType::IncludesRecipe => EdgeType::IncludesRecipe,
        RelationType::DeclaresResource => EdgeType::DeclaresResource,
        RelationType::UsesTemplate => EdgeType::UsesTemplate,
        RelationType::DefinesAttribute => EdgeType::DefinesAttribute,
        RelationType::NotifiesResource => EdgeType::NotifiesResource,
        RelationType::DependsOnModule => EdgeType::DependsOnModule,
        RelationType::IncludesClass => EdgeType::IncludesClass,
        RelationType::InheritsClass => EdgeType::InheritsClass,
        RelationType::RequiresResource => EdgeType::RequiresResource,
        RelationType::UsesFact => EdgeType::UsesFact,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbuilder_plugin_api::{ConfigValueType, SourceLocation, SymbolType};

    fn sample_symbol() -> Symbol {
        Symbol {
            name: "main".to_string(),
            symbol_type: SymbolType::Function,
            qualified_name: None,
            location: SourceLocation {
                file: "src/main.rs".to_string(),
                start_line: 1,
                end_line: 3,
                start_column: 0,
                end_column: 1,
            },
            signature: Some("fn main()".to_string()),
            return_type: None,
            parameters: vec![],
            fields: vec![],
            modifiers: vec![],
            documentation: None,
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn add_symbol_with_body_sets_token_bloom() {
        let mut builder = GraphBuilder::new();
        let file_id = builder.ensure_file_node(Path::new("src/main.rs"));
        let symbol = sample_symbol();
        builder.add_symbol_with_body(&symbol, file_id, Some("let port = ntohs(raw);"));
        let node = builder
            .nodes()
            .iter()
            .find(|n| n.name == symbol.name)
            .unwrap();
        let bloom = node.token_bloom.expect("token bloom");
        assert!(rbuilder_graph::keyword_in_bloom("ntohs", &bloom));
    }

    #[test]
    fn test_add_symbol_creates_file_and_symbol_nodes() {
        let mut builder = GraphBuilder::new();
        let file_id = builder.ensure_file_node(Path::new("src/main.rs"));
        builder.add_symbol(&sample_symbol(), file_id);

        assert_eq!(builder.node_count(), 2);
        assert_eq!(builder.edge_count(), 2);
    }

    #[test]
    fn test_add_symbol_materializes_fields() {
        let mut builder = GraphBuilder::new();
        let file_id = builder.ensure_file_node(Path::new("OrderDTO.java"));
        let mut symbol = sample_symbol();
        symbol.name = "OrderDTO".to_string();
        symbol.symbol_type = SymbolType::Class;
        symbol.location.file = "OrderDTO.java".to_string();
        symbol.fields = vec![rbuilder_plugin_api::Field {
            name: "status".to_string(),
            field_type: Some("String".to_string()),
            visibility: Some("private".to_string()),
        }];
        builder.add_symbol(&symbol, file_id);

        let field = builder
            .nodes()
            .iter()
            .find(|n| n.name == "status" && n.node_type == NodeType::Variable)
            .expect("field variable node");
        assert_eq!(
            field.properties.get("member_of").map(String::as_str),
            Some("OrderDTO")
        );
        assert_eq!(
            field.properties.get("field_type").map(String::as_str),
            Some("String")
        );
        let owner = builder
            .nodes()
            .iter()
            .find(|n| n.name == "OrderDTO")
            .unwrap();
        assert!(builder.edges.iter().any(|e| {
            e.from == owner.id && e.to == field.id && e.edge_type == EdgeType::Contains
        }));
    }

    #[test]
    fn test_add_config_key() {
        let mut builder = GraphBuilder::new();
        let file_id = builder.ensure_file_node(Path::new("config.yaml"));
        let key = ConfigKey {
            key_path: "database.host".to_string(),
            value: "localhost".to_string(),
            value_type: ConfigValueType::String,
            location: SourceLocation {
                file: "config.yaml".to_string(),
                start_line: 1,
                end_line: 1,
                start_column: 0,
                end_column: 0,
            },
        };

        builder.add_config_key(&key, file_id);
        assert_eq!(builder.node_count(), 2);
    }

    #[test]
    fn test_link_config_usage_env_var() {
        let mut builder = GraphBuilder::new();
        let file_id = builder.ensure_file_node(Path::new("src/main.rs"));
        builder.add_symbol(&sample_symbol(), file_id);

        builder.link_config_usage("src/main.rs", 1, "DB_HOST", ConfigUsageKind::EnvVar);

        assert!(builder.node_count() >= 3);
        assert!(builder
            .edges
            .iter()
            .any(|e| e.edge_type == EdgeType::UsesConfig));
    }

    fn function_symbol(file: &str, name: &str, qualified: &str) -> Symbol {
        Symbol {
            name: name.to_string(),
            symbol_type: SymbolType::Function,
            qualified_name: Some(qualified.to_string()),
            location: SourceLocation {
                file: file.to_string(),
                start_line: 1,
                end_line: 3,
                start_column: 0,
                end_column: 1,
            },
            signature: None,
            return_type: None,
            parameters: vec![],
            fields: vec![],
            modifiers: vec![],
            documentation: None,
            metadata: serde_json::json!({}),
        }
    }

    /// Ambiguous FQN must not resolve as a single definitive UUID (#27).
    #[test]
    fn qe_duplicate_qualified_name_must_not_collapse_index() {
        let mut builder = GraphBuilder::new();
        let a = builder.ensure_file_node(Path::new("a.rs"));
        let b = builder.ensure_file_node(Path::new("b.rs"));
        builder.add_symbol(&function_symbol("a.rs", "transform", "Helper.transform"), a);
        builder.add_symbol(&function_symbol("b.rs", "transform", "Helper.transform"), b);
        builder.build_resolution_indexes();

        let nodes_with_fqn = builder
            .nodes()
            .iter()
            .filter(|n| n.qualified_name.as_deref() == Some("Helper.transform"))
            .count();
        assert_eq!(nodes_with_fqn, 2, "both nodes must survive ingest");

        let suffix_n = builder
            .symbols_by_suffix
            .get("Helper.transform")
            .map(|v| v.len())
            .unwrap_or(0);
        assert!(
            suffix_n >= 2,
            "suffix index should retain both UUIDs (got {suffix_n})"
        );

        let qn_n = builder
            .symbols_by_qualified
            .get("Helper.transform")
            .map(|v| v.len())
            .unwrap_or(0);
        assert!(
            qn_n >= 2,
            "qualified index should retain both UUIDs (got {qn_n})"
        );

        let resolved = builder.resolve_symbol_tracked(
            "transform",
            "caller.rs",
            Some("Helper.transform"),
            None,
        );
        assert!(
            resolved.is_none(),
            "QE: duplicate FQN must not resolve to a single UUID via qualified hint \
             (got {resolved:?}); see rbuilder-tests/correctness/QE.md"
        );
    }

    /// Suffix multi-match must not silently pick `.first()` (#27).
    #[test]
    fn qe_suffix_multimatch_must_not_pick_first_silently() {
        let mut builder = GraphBuilder::new();
        let a = builder.ensure_file_node(Path::new("pkg/a.rs"));
        let b = builder.ensure_file_node(Path::new("pkg/b.rs"));
        builder.add_symbol(&function_symbol("pkg/a.rs", "twin", "alpha::twin"), a);
        builder.add_symbol(&function_symbol("pkg/b.rs", "twin", "beta::twin"), b);
        builder.build_resolution_indexes();

        let candidates = builder
            .symbols_by_suffix
            .get("twin")
            .map(|v| v.len())
            .unwrap_or(0);
        assert!(
            candidates >= 2,
            "expected ≥2 suffix candidates for twin, got {candidates}"
        );

        let resolved = builder.resolve_symbol_tracked("twin", "other.rs", None, None);
        assert!(
            resolved.is_none(),
            "QE: fuzzy suffix multi-match must not return Some(uuid) without signaling ambiguity \
             (got {resolved:?}); see rbuilder-tests/correctness/QE.md"
        );
    }
}
