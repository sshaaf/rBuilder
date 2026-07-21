//! `rbuilder export` — graph serialization.

use super::args::ExportFormat;
use super::context::CliContext;
use crate::export::{export_graphml, generate_dot, generate_mermaid, select_subgraph};
use crate::export::{GraphvizOptions, MermaidOptions};
use anyhow::{bail, Result};
use rbuilder_graph::export::GraphSnapshot;
use rbuilder_graph::schema::GRAPH_SCHEMA_VERSION;

pub struct ExportArgs {
    pub export_format: ExportFormat,
    pub export_output: String,
    pub query: String,
}

pub fn run(ctx: &CliContext, args: ExportArgs) -> Result<()> {
    let graph = ctx.load_graph()?;
    let backend = graph.backend();
    let query = args.query.trim();
    if query.is_empty() {
        bail!("--query must not be empty (use `all` for the full graph)");
    }

    let (node_count, edge_count) = match args.export_format {
        ExportFormat::Json if query == "all" => {
            std::fs::write(&args.export_output, graph.export_json()?)?;
            (graph.node_count(), graph.edge_count())
        }
        ExportFormat::Json => {
            let sg = select_subgraph(backend, query, None)?;
            if sg.nodes.is_empty() {
                bail!("No nodes matched query: {query}");
            }
            let node_count = sg.nodes.len();
            let edge_count = sg.edges.len();
            let snapshot = GraphSnapshot {
                version: env!("CARGO_PKG_VERSION").to_string(),
                schema_version: GRAPH_SCHEMA_VERSION,
                nodes: sg.nodes,
                edges: sg.edges,
            };
            std::fs::write(
                &args.export_output,
                serde_json::to_string_pretty(&snapshot)?,
            )?;
            (node_count, edge_count)
        }
        ExportFormat::Graphml => {
            let content = export_graphml(backend, query)?;
            let counts = filtered_counts(backend, query)?;
            std::fs::write(&args.export_output, content)?;
            counts
        }
        ExportFormat::Graphviz => {
            let dot = generate_dot(backend, query, GraphvizOptions::default(), None)
                .map_err(|e| anyhow::anyhow!(e))?;
            let counts = filtered_counts(backend, query)?;
            std::fs::write(&args.export_output, dot)?;
            counts
        }
        ExportFormat::Mermaid => {
            let mmd = generate_mermaid(backend, query, MermaidOptions::default())
                .map_err(|e| anyhow::anyhow!(e))?;
            let counts = filtered_counts(backend, query)?;
            std::fs::write(&args.export_output, mmd)?;
            counts
        }
    };

    if ctx.output.is_none() {
        println!(
            "Exported {node_count} nodes, {edge_count} edges -> {}",
            args.export_output
        );
    }
    Ok(())
}

fn filtered_counts(
    backend: &rbuilder_graph::backend::MemoryBackend,
    query: &str,
) -> Result<(usize, usize)> {
    if query == "all" {
        Ok((backend.node_count(), backend.edge_count()))
    } else {
        let sg = select_subgraph(backend, query, None)?;
        Ok((sg.nodes.len(), sg.edges.len()))
    }
}
