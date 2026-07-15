//! Export per-node analysis scores for the Functions dashboard tab.

use crate::export_context::{resolve_analysis, DashboardExportContext};
use crate::metagraph::COMMUNITY_ONLY_THRESHOLD;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use uuid::Uuid;

pub const FUNCTION_METRICS_FILE: &str = "function_metrics.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMetricRow {
    pub index: u32,
    pub pagerank: f32,
    pub betweenness: f32,
    pub harmonic: f32,
    pub blast: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub community_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMetricsPayload {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<FunctionMetricRow>,
    /// When set, per-function rows were omitted for large graphs (WASM/metagraph only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sparse_mode: Option<String>,
}

pub fn export_function_metrics(
    snapshot_path: &Path,
    out_dir: &Path,
    source_node_count: u64,
    ctx: DashboardExportContext<'_>,
    uuid_to_index: &HashMap<Uuid, u32>,
) -> Result<(), String> {
    if !snapshot_path.is_file() {
        write_empty(out_dir)?;
        return Ok(());
    }

    if source_node_count >= COMMUNITY_ONLY_THRESHOLD {
        write_community_only(out_dir)?;
        return Ok(());
    }

    let repo_root = snapshot_path
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| "invalid snapshot path".to_string())?;
    let results = resolve_analysis(&ctx, repo_root)?;

    let centrality = results.centrality.as_ref();
    let blast = results.blast_radius.as_ref();

    let mut rows = Vec::new();
    for compact_id in 0..results.node_count() {
        let Some(uuid) = results.get_uuid(compact_id as u32) else {
            continue;
        };
        let Some(&index) = uuid_to_index.get(&uuid) else {
            continue;
        };

        let pagerank = centrality
            .and_then(|c| c.pagerank.get(compact_id).copied())
            .unwrap_or(0.0);
        let betweenness = centrality
            .and_then(|c| c.betweenness.get(compact_id).copied())
            .unwrap_or(0.0);
        let harmonic = centrality
            .and_then(|c| c.harmonic.get(compact_id).copied())
            .unwrap_or(0.0);
        let blast_score = blast
            .and_then(|b| b.scores.get(compact_id).copied())
            .unwrap_or(0.0);
        let community_id = results
            .community
            .as_ref()
            .and_then(|c| c.get(compact_id as u32))
            .map(|id| id as u32);

        if pagerank <= 0.0
            && betweenness <= 0.0
            && harmonic <= 0.0
            && blast_score <= 0.0
            && community_id.is_none()
        {
            continue;
        }

        rows.push(FunctionMetricRow {
            index,
            pagerank,
            betweenness,
            harmonic,
            blast: blast_score,
            community_id,
        });
    }

    rows.sort_by_key(|a| a.index);
    write_payload(out_dir, rows, None)
}

fn write_payload(
    out_dir: &Path,
    rows: Vec<FunctionMetricRow>,
    sparse_mode: Option<String>,
) -> Result<(), String> {
    let payload = FunctionMetricsPayload {
        schema_version: if sparse_mode.is_some() { 3 } else { 2 },
        rows,
        sparse_mode,
    };
    let json = {
        let start = std::time::Instant::now();
        let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
        tracing::info!(
            target: "profile",
            file = FUNCTION_METRICS_FILE,
            serialize_secs = start.elapsed().as_secs_f64(),
            json_bytes = json.len(),
            rows = payload.rows.len(),
            "[profile] save_dashboard json serialize"
        );
        json
    };
    let write_start = std::time::Instant::now();
    fs::write(out_dir.join(FUNCTION_METRICS_FILE), json).map_err(|e| e.to_string())?;
    tracing::info!(
        target: "profile",
        file = FUNCTION_METRICS_FILE,
        write_secs = write_start.elapsed().as_secs_f64(),
        "[profile] save_dashboard json write"
    );
    Ok(())
}

fn write_empty(out_dir: &Path) -> Result<(), String> {
    write_payload(out_dir, Vec::new(), None)
}

fn write_community_only(out_dir: &Path) -> Result<(), String> {
    write_payload(
        out_dir,
        Vec::new(),
        Some("community_only".into()),
    )
}
