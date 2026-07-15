//! Opt-in semantic search over function symbols using binary-quantized embeddings.
//!
//! Embeddings are stored separately from [`AnalysisResults`] in `.rbuilder/semantic_index.bin`
//! so the default discover path stays lean. Default embedder is sign-hash; ONNX is available
//! with the `semantic-onnx` feature.

use crate::semantic_embedder::{embedder_for_index, OnnxReloadOptions, SemanticEmbedder};
use crate::semantic_extract::extract_body_tokens_for_node;
use rbuilder_error::Result;
use rbuilder_graph::backend::MemoryBackend;
use rbuilder_graph::schema::{Node, NodeType};
use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Current on-disk schema version for [`SemanticIndex`].
pub const SEMANTIC_INDEX_SCHEMA_VERSION: u32 = 2;

/// Default filename under `.rbuilder/`.
pub const SEMANTIC_INDEX_FILE: &str = "semantic_index.bin";

/// Default float dimensions before sign quantization (128 bytes per vector).
pub const DEFAULT_EMBEDDING_DIMENSIONS: usize = 1024;

/// Identifier for the built-in deterministic hash embedder.
pub const SIGN_HASH_MODEL_ID: &str = "sign-hash-v1";

/// One indexed function symbol and its metadata for query display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticEntry {
    /// Graph node UUID.
    pub node_id: Uuid,
    /// Short symbol name.
    pub name: String,
    /// Fully qualified name when known.
    pub qualified_name: Option<String>,
    /// Source file path when known.
    pub file_path: Option<String>,
    /// BLAKE3 body hash at index time (incremental reuse).
    #[serde(default)]
    pub code_hash: Option<String>,
}

/// Bit-packed semantic index over function nodes only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticIndex {
    /// Format version for forward-compatible load.
    pub schema_version: u32,
    /// Embedder identifier (e.g. `sign-hash-v1`).
    pub model_id: String,
    /// Float dimensions before quantization.
    pub dimensions: usize,
    /// Graph snapshot digest when indexed (optional invalidation).
    pub graph_digest: Option<String>,
    /// ONNX model path when `model_id` starts with `onnx:` or `code-daemon:v1`.
    #[serde(default)]
    pub model_path: Option<String>,
    /// SentencePiece path for ONNX embedders (optional; sibling auto-detect at index time).
    #[serde(default)]
    pub tokenizer_path: Option<String>,
    /// Row order matches contiguous slices of [`Self::binary_embeddings`].
    pub entries: Vec<SemanticEntry>,
    /// Flat bit-packed rows: `entries.len() * packed_bytes(dimensions)`.
    pub binary_embeddings: Vec<u8>,
}

impl SemanticIndex {
    /// Default path under a repository root.
    pub fn default_path(repo_root: &Path) -> PathBuf {
        repo_root.join(".rbuilder").join(SEMANTIC_INDEX_FILE)
    }

    /// Bytes per quantized vector row.
    pub fn bytes_per_vector(&self) -> usize {
        packed_bytes(self.dimensions)
    }

    /// Number of indexed functions.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when no functions were indexed.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Binary embedding for row `index`.
    pub fn embedding_row(&self, index: usize) -> Option<&[u8]> {
        let stride = self.bytes_per_vector();
        let start = index.checked_mul(stride)?;
        self.binary_embeddings.get(start..start + stride)
    }

    /// Save index to disk (bincode, same pattern as [`crate::results::AnalysisResults`]).
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(path)?;
        bincode::serialize_into(file, self).map_err(serde_err)
    }

    /// Load index from disk.
    pub fn load(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        bincode::deserialize_from(file).map_err(serde_err)
    }

    /// Load when present; `Ok(None)` if the file is missing.
    pub fn open_if_exists(repo_root: &Path) -> Result<Option<Self>> {
        let path = Self::default_path(repo_root);
        if !path.is_file() {
            return Ok(None);
        }
        Self::load(&path).map(Some)
    }
}

/// Stats from an index build (full or incremental).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SemanticBuildStats {
    pub total: usize,
    pub reused: usize,
    pub embedded: usize,
    pub removed: usize,
}

/// Options controlling semantic index construction.
#[derive(Debug, Clone)]
pub struct SemanticBuildOptions {
    pub dimensions: usize,
    pub graph_digest: Option<String>,
    pub incremental: bool,
    pub existing: Option<SemanticIndex>,
    pub model_path: Option<String>,
    pub tokenizer_path: Option<String>,
    /// Repository root for on-demand body slicing during indexing.
    pub repo_root: Option<PathBuf>,
}

impl SemanticBuildOptions {
    /// Defaults for a fresh full build.
    pub fn fresh(dimensions: usize, graph_digest: Option<String>) -> Self {
        Self {
            dimensions,
            graph_digest,
            incremental: false,
            existing: None,
            model_path: None,
            tokenizer_path: None,
            repo_root: None,
        }
    }
}

/// Build a semantic index from all `Function` nodes using the given embedder.
pub fn build_index(
    backend: &MemoryBackend,
    embedder: &dyn SemanticEmbedder,
    options: SemanticBuildOptions,
) -> Result<(SemanticIndex, SemanticBuildStats)> {
    let row_bytes = packed_bytes(options.dimensions);
    let mut entries = Vec::new();
    let mut binary_embeddings = Vec::new();
    let mut stats = SemanticBuildStats::default();

    let mut reuse_by_id: HashMap<Uuid, (SemanticEntry, Vec<u8>)> = HashMap::new();
    if options.incremental {
        if let Some(existing) = &options.existing {
            if existing.dimensions != options.dimensions || existing.model_id != embedder.model_id()
            {
                // Dimension/model mismatch — full rebuild.
            } else {
                let stride = existing.bytes_per_vector();
                for (row, entry) in existing.entries.iter().enumerate() {
                    if let Some(slice) = existing
                        .binary_embeddings
                        .get(row * stride..row * stride + stride)
                    {
                        reuse_by_id.insert(entry.node_id, (entry.clone(), slice.to_vec()));
                    }
                }
            }
        }
    }

    let repo_root = options.repo_root.as_deref();
    let mut functions: Vec<(SemanticEntry, String)> = Vec::new();
    backend.for_each_node(|node| {
        if let Some(text) = embed_text_for_function(node, repo_root) {
            functions.push((
                SemanticEntry {
                    node_id: node.id,
                    name: node.name.clone(),
                    qualified_name: node.qualified_name.clone(),
                    file_path: node.file_path.clone(),
                    code_hash: node.code_hash.clone(),
                },
                text,
            ));
        }
    })?;

    let mut seen = HashSet::new();
    for (fresh_entry, text) in functions {
        seen.insert(fresh_entry.node_id);
        stats.total += 1;

        if options.incremental {
            if let Some((old_entry, old_bits)) = reuse_by_id.get(&fresh_entry.node_id) {
                if old_entry.code_hash == fresh_entry.code_hash {
                    entries.push(old_entry.clone());
                    binary_embeddings.extend_from_slice(old_bits);
                    stats.reused += 1;
                    continue;
                }
            }
        }

        let floats = embedder.embed(&text)?;
        entries.push(fresh_entry);
        binary_embeddings.extend_from_slice(&quantize_binary(&floats));
        stats.embedded += 1;
    }

    if options.incremental {
        if let Some(existing) = &options.existing {
            stats.removed = existing
                .entries
                .iter()
                .filter(|entry| !seen.contains(&entry.node_id))
                .count();
        }
    }

    debug_assert_eq!(binary_embeddings.len(), entries.len() * row_bytes);

    let index = SemanticIndex {
        schema_version: SEMANTIC_INDEX_SCHEMA_VERSION,
        model_id: embedder.model_id().to_string(),
        dimensions: options.dimensions,
        graph_digest: options.graph_digest,
        model_path: options.model_path,
        tokenizer_path: options.tokenizer_path,
        entries,
        binary_embeddings,
    };

    Ok((index, stats))
}

/// Build a semantic index from all `Function` nodes (sign-hash, non-incremental).
pub fn build_from_backend(
    backend: &MemoryBackend,
    dimensions: usize,
    graph_digest: Option<String>,
) -> Result<SemanticIndex> {
    let embedder = crate::semantic_embedder::SignHashEmbedder::new(dimensions);
    let (index, _stats) = build_index(
        backend,
        &embedder,
        SemanticBuildOptions::fresh(dimensions, graph_digest),
    )?;
    Ok(index)
}

/// Collect embeddable text for a function node (declaration metadata only).
pub fn embed_text_for_node(node: &Node) -> Option<String> {
    embed_text_for_function(node, None)
}

/// Collect embeddable text for a function node, optionally including body tokens.
pub fn embed_text_for_function(node: &Node, repo_root: Option<&Path>) -> Option<String> {
    if node.node_type != NodeType::Function {
        return None;
    }

    let mut parts = Vec::new();
    if let Some(qn) = &node.qualified_name {
        parts.push(qn.clone());
    } else {
        parts.push(node.name.clone());
    }
    if let Some(sig) = node.signature_text() {
        parts.push(sig.to_string());
    }
    if let Some(ret) = node.return_type_text() {
        parts.push(format!("returns {ret}"));
    }
    if let Some(doc) = node.get_property("documentation") {
        parts.push(doc.to_string());
    }

    if let Some(root) = repo_root {
        if let Ok(body_tokens) = extract_body_tokens_for_node(root, node) {
            let mut token_list: Vec<String> = body_tokens.into_iter().collect();
            token_list.sort_unstable();
            parts.extend(token_list);
        }
    }

    Some(parts.join(" "))
}

/// Deterministic sign-hash embedding (bag-of-tokens → sparse signed vector).
pub fn sign_hash_embed(text: &str, dimensions: usize) -> Vec<f32> {
    let mut vec = vec![0f32; dimensions];
    for token in tokenize(text) {
        let primary = fnv1a(token.as_bytes());
        let secondary = fnv1a(&[token.as_bytes(), b"#2"].concat());
        let sign = if primary & 1 == 0 { 1.0 } else { -1.0 };
        vec[primary as usize % dimensions] += sign;
        vec[secondary as usize % dimensions] += sign * 0.5;
    }
    vec
}

/// Sign-quantize a float vector into little-endian bit-packed bytes.
pub fn quantize_binary(floats: &[f32]) -> Vec<u8> {
    let mut out = vec![0u8; packed_bytes(floats.len())];
    for (i, value) in floats.iter().enumerate() {
        if *value >= 0.0 {
            out[i / 8] |= 1 << (i % 8);
        }
    }
    out
}

/// Hamming distance between two equal-length bit-packed vectors.
///
/// Processes 64-bit words so LLVM can lower XOR/`popcnt` efficiently.
pub fn hamming_distance(a: &[u8], b: &[u8]) -> u32 {
    debug_assert_eq!(a.len(), b.len(), "Hamming distance requires equal-length vectors");

    let word_bytes = a.len() - (a.len() % 8);
    let mut total = 0u32;

    for (chunk_a, chunk_b) in a[..word_bytes]
        .chunks_exact(8)
        .zip(b[..word_bytes].chunks_exact(8))
    {
        let word_a = u64::from_ne_bytes(chunk_a.try_into().expect("8-byte chunk"));
        let word_b = u64::from_ne_bytes(chunk_b.try_into().expect("8-byte chunk"));
        total += (word_a ^ word_b).count_ones();
    }

    for i in word_bytes..a.len() {
        total += (a[i] ^ b[i]).count_ones();
    }

    total
}

/// Return up to `k` nearest rows by Hamming distance (smallest first).
///
/// Uses a bounded max-heap so cost is O(n log k), not O(n log n).
pub fn hamming_top_k(index: &SemanticIndex, query: &[u8], k: usize) -> Vec<(usize, u32)> {
    if k == 0 || index.is_empty() {
        return Vec::new();
    }

    let stride = index.bytes_per_vector();
    debug_assert_eq!(index.binary_embeddings.len(), index.len() * stride);

    let mut heap: BinaryHeap<(u32, usize)> = BinaryHeap::with_capacity(k.saturating_add(1));
    for (row, chunk) in index.binary_embeddings.chunks_exact(stride).enumerate() {
        let dist = hamming_distance(query, chunk);
        if heap.len() < k {
            heap.push((dist, row));
        } else if let Some(&(worst, _)) = heap.peek() {
            if dist < worst {
                heap.pop();
                heap.push((dist, row));
            }
        }
    }

    let mut hits: Vec<(usize, u32)> = heap.into_iter().map(|(d, i)| (i, d)).collect();
    hits.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    hits
}

/// Embed query text and search the index (sign-hash only; prefer [`query_index_with_embedder`]).
pub fn query_index(index: &SemanticIndex, text: &str, k: usize) -> Vec<SemanticHit> {
    query_index_with_embedder(index, text, k, &OnnxReloadOptions::default()).unwrap_or_default()
}

/// Embed query text and search using the embedder matching the index.
pub fn query_index_with_embedder(
    index: &SemanticIndex,
    text: &str,
    k: usize,
    reload: &OnnxReloadOptions,
) -> Result<Vec<SemanticHit>> {
    let embedder = embedder_for_index(index, reload)?;
    let query_bits = embedder.embed_binary(text)?;
    Ok(hamming_top_k(index, &query_bits, k)
        .into_iter()
        .filter_map(|(row, distance)| {
            let entry = index.entries.get(row)?;
            Some(SemanticHit {
                row,
                distance,
                entry: entry.clone(),
                fused_score: None,
            })
        })
        .collect())
}

/// One query hit with Hamming distance.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticHit {
    /// Row index in the index tables.
    pub row: usize,
    /// Hamming distance to the query (lower is better).
    pub distance: u32,
    /// Indexed function metadata.
    pub entry: SemanticEntry,
    /// Late-fusion score when two-stage ranking is enabled (higher is better).
    pub fused_score: Option<f64>,
}

fn packed_bytes(dimensions: usize) -> usize {
    dimensions.div_ceil(8)
}

fn tokenize(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|token| !token.is_empty())
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn serde_err(err: impl std::fmt::Display) -> rbuilder_error::Error {
    rbuilder_error::Error::SerdeError(format!("semantic index: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbuilder_graph::backend::GraphBackend;
    use rbuilder_graph::schema::Node;
    use std::collections::HashSet;

    #[test]
    fn quantize_round_trip_sign() {
        let floats = vec![1.0, -2.0, 0.0, 3.0];
        let bits = quantize_binary(&floats);
        assert_eq!(bits, vec![0b00001101]);
    }

    #[test]
    fn hamming_distance_counts_bit_flips() {
        assert_eq!(hamming_distance(&[0b1111_0000], &[0b1010_0000]), 2);
    }

    #[test]
    fn hamming_distance_matches_byte_wise_reference() {
        fn byte_wise(a: &[u8], b: &[u8]) -> u32 {
            a.iter()
                .zip(b.iter())
                .map(|(left, right)| (left ^ right).count_ones())
                .sum()
        }

        let a: Vec<u8> = (0..37).map(|i| (i * 17) as u8).collect();
        let b: Vec<u8> = (0..37).map(|i| (i * 31) as u8).collect();
        assert_eq!(hamming_distance(&a, &b), byte_wise(&a, &b));

        let packed = vec![0u8; packed_bytes(1024)];
        let mut flipped = packed.clone();
        flipped[0] = 0xFF;
        flipped[31] = 0x0F;
        assert_eq!(hamming_distance(&packed, &flipped), byte_wise(&packed, &flipped));
    }

    #[test]
    fn hamming_top_k_returns_smallest_distances() {
        let index = SemanticIndex {
            schema_version: SEMANTIC_INDEX_SCHEMA_VERSION,
            model_id: SIGN_HASH_MODEL_ID.into(),
            dimensions: 8,
            graph_digest: None,
            model_path: None,
            tokenizer_path: None,
            entries: (0..4)
                .map(|i| SemanticEntry {
                    node_id: Uuid::new_v4(),
                    name: format!("f{i}"),
                    qualified_name: None,
                    file_path: None,
                    code_hash: None,
                })
                .collect(),
            binary_embeddings: vec![
                0b0000_0000, // dist 0
                0b0000_0001, // dist 1
                0b0000_0011, // dist 2
                0b1111_1111, // dist 8
            ],
        };
        let query = vec![0b0000_0000];
        let top = hamming_top_k(&index, &query, 2);
        assert_eq!(top, vec![(0, 0), (1, 1)]);
    }

    #[test]
    fn sign_hash_embed_is_deterministic() {
        let a = sign_hash_embed("authenticate user token", 64);
        let b = sign_hash_embed("authenticate user token", 64);
        assert_eq!(a, b);
        assert_ne!(a, sign_hash_embed("revoke user token", 64));
    }

    #[test]
    fn embed_text_for_function_includes_signature() {
        let node = Node::new(NodeType::Function, "run".into())
            .with_qualified_name("auth::run".into())
            .with_signature("async fn run(token: &str) -> bool");
        let text = embed_text_for_node(&node).unwrap();
        assert!(text.contains("auth::run"));
        assert!(text.contains("async fn run"));
    }

    #[test]
    fn build_and_query_from_backend() {
        let mut backend = MemoryBackend::new();
        let n1 = Node::new(NodeType::Function, "authenticate".into())
            .with_qualified_name("auth::authenticate".into())
            .with_signature("fn authenticate(token: &str) -> bool");
        let n2 = Node::new(NodeType::Function, "revoke".into())
            .with_qualified_name("auth::revoke".into())
            .with_signature("fn revoke(token: &str)");
        let class = Node::new(NodeType::Class, "AuthService".into());
        backend.insert_node(n1.clone()).unwrap();
        backend.insert_node(n2.clone()).unwrap();
        backend.insert_node(class).unwrap();

        let index = build_from_backend(&backend, 128, None).unwrap();
        assert_eq!(index.len(), 2);

        let hits = query_index(&index, "authenticate bearer token", 2);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].entry.node_id, n1.id);
        assert!(hits[0].distance <= hits[1].distance);
    }

    #[test]
    fn save_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SEMANTIC_INDEX_FILE);
        let index = SemanticIndex {
            schema_version: SEMANTIC_INDEX_SCHEMA_VERSION,
            model_id: SIGN_HASH_MODEL_ID.into(),
            dimensions: 16,
            graph_digest: Some("abc".into()),
            model_path: None,
            tokenizer_path: None,
            entries: vec![SemanticEntry {
                node_id: Uuid::new_v4(),
                name: "main".into(),
                qualified_name: None,
                file_path: Some("src/main.rs".into()),
                code_hash: Some("abc".into()),
            }],
            binary_embeddings: vec![0b1010_1010, 0b0101_0101],
        };
        index.save(&path).unwrap();
        let loaded = SemanticIndex::load(&path).unwrap();
        assert_eq!(loaded, index);
    }

    #[test]
    fn incremental_reuses_unchanged_code_hash() {
        let mut backend = MemoryBackend::new();
        let n1 = Node::new(NodeType::Function, "authenticate".into())
            .with_code_hash("h1")
            .with_signature("fn authenticate()");
        let n2 = Node::new(NodeType::Function, "revoke".into())
            .with_code_hash("h2")
            .with_signature("fn revoke()");
        backend.insert_node(n1).unwrap();
        backend.insert_node(n2.clone()).unwrap();

        let embedder = crate::semantic_embedder::SignHashEmbedder::new(64);
        let (index, stats) = build_index(
            &backend,
            &embedder,
            SemanticBuildOptions::fresh(64, None),
        )
        .unwrap();
        assert_eq!(stats.embedded, 2);

        let (index2, stats2) = build_index(
            &backend,
            &embedder,
            SemanticBuildOptions {
                dimensions: 64,
                graph_digest: None,
                incremental: true,
                existing: Some(index),
                model_path: None,
                tokenizer_path: None,
                repo_root: None,
            },
        )
        .unwrap();
        assert_eq!(stats2.reused, 2);
        assert_eq!(stats2.embedded, 0);
        assert_eq!(index2.len(), 2);

        // Change one function body hash — only that row re-embeds.
        let mut n2_updated = n2;
        n2_updated.code_hash = Some("h2-v2".into());
        n2_updated.signature = Some("fn revoke(token: &str)".into());
        backend.insert_node(n2_updated).unwrap();
        let (index3, stats3) = build_index(
            &backend,
            &embedder,
            SemanticBuildOptions {
                dimensions: 64,
                graph_digest: None,
                incremental: true,
                existing: Some(index2),
                model_path: None,
                tokenizer_path: None,
                repo_root: None,
            },
        )
        .unwrap();
        assert_eq!(stats3.reused, 1);
        assert_eq!(stats3.embedded, 1);
        assert_eq!(index3.len(), 2);
    }

    #[test]
    fn body_tokens_improve_retrieval_for_implementation_vocabulary() {
        let dir = tempfile::tempdir().unwrap();
        let rel = "src/net.rs";
        let abs = dir.path().join(rel);
        std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
        std::fs::write(
            &abs,
            "fn cryptic_a() {}\n\nfn cryptic_b() {\n    let port = ntohs(raw);\n}\n",
        )
        .unwrap();

        let mut backend = MemoryBackend::new();
        let opaque = Node::new(NodeType::Function, "cryptic_a".into())
            .with_file_path(rel.into())
            .with_location(1, 1);
        let helper = Node::new(NodeType::Function, "cryptic_b".into())
            .with_file_path(rel.into())
            .with_location(3, 5)
            .with_code_hash("body-v1");
        backend.insert_node(opaque.clone()).unwrap();
        backend.insert_node(helper.clone()).unwrap();

        let index_no_body = build_index(
            &backend,
            &crate::semantic_embedder::SignHashEmbedder::new(128),
            SemanticBuildOptions {
                dimensions: 128,
                graph_digest: None,
                incremental: false,
                existing: None,
                model_path: None,
                tokenizer_path: None,
                repo_root: None,
            },
        )
        .unwrap()
        .0;

        let index_with_body = build_index(
            &backend,
            &crate::semantic_embedder::SignHashEmbedder::new(128),
            SemanticBuildOptions {
                dimensions: 128,
                graph_digest: None,
                incremental: false,
                existing: None,
                model_path: None,
                tokenizer_path: None,
                repo_root: Some(dir.path().to_path_buf()),
            },
        )
        .unwrap()
        .0;

        let hits_no_body = query_index(&index_no_body, "ntohs packet port", 2);
        let hits_with_body = query_index(&index_with_body, "ntohs packet port", 2);

        let dist_no_body = hits_no_body
            .iter()
            .find(|hit| hit.entry.node_id == helper.id)
            .map(|hit| hit.distance)
            .expect("helper indexed");
        let dist_with_body = hits_with_body
            .iter()
            .find(|hit| hit.entry.node_id == helper.id)
            .map(|hit| hit.distance)
            .expect("helper indexed");

        assert!(dist_with_body < dist_no_body);
        assert_eq!(hits_with_body[0].entry.node_id, helper.id);
    }

    #[test]
    fn hamming_top_k_covers_all_rows_when_k_large() {
        let dims = 8;
        let rows = 5usize;
        let mut bits = Vec::new();
        for i in 0..rows {
            bits.push(i as u8);
        }
        let index = SemanticIndex {
            schema_version: SEMANTIC_INDEX_SCHEMA_VERSION,
            model_id: SIGN_HASH_MODEL_ID.into(),
            dimensions: dims,
            graph_digest: None,
            model_path: None,
            tokenizer_path: None,
            entries: (0..rows)
                .map(|i| SemanticEntry {
                    node_id: Uuid::new_v4(),
                    name: format!("f{i}"),
                    qualified_name: None,
                    file_path: None,
                    code_hash: None,
                })
                .collect(),
            binary_embeddings: bits,
        };
        let hits = hamming_top_k(&index, &[0u8], rows + 10);
        let rows_seen: HashSet<_> = hits.iter().map(|(r, _)| *r).collect();
        assert_eq!(rows_seen.len(), rows);
    }
}
