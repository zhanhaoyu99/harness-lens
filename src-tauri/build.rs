fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "choose_workspace",
        "load_default_workspace",
        "rescan_workspace",
        "list_context_snapshots",
        "capture_context_snapshot",
        "load_context_snapshot",
        "compare_context_snapshots",
        "clear_context_snapshot_history",
        "inspect_runtime",
        "load_runtime_run",
        "load_memory_artifact",
        "save_memory_artifact",
        "open_artifact",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("failed to build Harness Lens");
}
