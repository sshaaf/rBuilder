//! Stage timing helpers for save-path profiling (`RUST_LOG=profile=info`).

use std::time::Instant;

/// Run `f`, log wall seconds under `[profile] save_dashboard stage`.
pub fn profile_stage<T, F>(stage: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let out = f();
    tracing::info!(
        target: "profile",
        stage,
        secs = start.elapsed().as_secs_f64(),
        "[profile] save_dashboard stage"
    );
    out
}

/// Log JSON serialize vs file write for one artifact.
pub fn profile_json_write(path: &std::path::Path, json: &str) -> Result<(), String> {
    let write_start = Instant::now();
    std::fs::write(path, json.as_bytes()).map_err(|e| e.to_string())?;
    tracing::info!(
        target: "profile",
        file = %path.file_name().and_then(|s| s.to_str()).unwrap_or("?"),
        json_bytes = json.len(),
        write_secs = write_start.elapsed().as_secs_f64(),
        "[profile] save_dashboard json write"
    );
    Ok(())
}
