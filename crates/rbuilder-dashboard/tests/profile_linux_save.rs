//! Manual profile: dashboard export on example/linux (requires prior discover).

use rbuilder_dashboard::export_dashboard_bundle;
use rbuilder_graph::snapshot::MmappedGraphSnapshot;
use std::path::Path;
use std::time::Instant;

#[test]
#[ignore = "manual: profile save_dashboard on example/linux artifact"]
fn profile_linux_dashboard_export() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("profile=info")
        .try_init();

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../example/linux");
    let snapshot_path = root.join(".rbuilder/graph.snapshot.bin");
    if !snapshot_path.is_file() {
        eprintln!("skip: {} missing", snapshot_path.display());
        return;
    }

    let hydrate_start = Instant::now();
    let snapshot = MmappedGraphSnapshot::open(&snapshot_path).expect("open snapshot");
    let backend = snapshot.hydrate_backend().expect("hydrate backend");
    tracing::info!(
        target: "profile",
        secs = hydrate_start.elapsed().as_secs_f64(),
        nodes = backend.node_count(),
        "[profile] hydrate_backend for dashboard export"
    );

    let dashboard = root.join(".rbuilder/dashboard");
    if dashboard.exists() {
        std::fs::remove_dir_all(&dashboard).expect("remove dashboard");
    }

    export_dashboard_bundle(&backend, &root, &snapshot_path).expect("export dashboard");
}
