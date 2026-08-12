use std::{collections::BTreeMap, env, path::PathBuf};

use harness_lens_lib::scanner;
use serde_json::json;

fn main() -> Result<(), String> {
    let workspace = env::args()
        .skip(1)
        .find(|argument| argument != "--")
        .map(PathBuf::from)
        .ok_or_else(|| "Usage: cargo run --example scan_summary -- <workspace>".to_string())?;
    let home =
        dirs::home_dir().ok_or_else(|| "Unable to locate the home directory.".to_string())?;
    let snapshot = scanner::scan(&workspace, &home)?;

    let mut by_kind = BTreeMap::new();
    let mut by_provider = BTreeMap::new();
    let mut by_resolution = BTreeMap::new();
    for artifact in &snapshot.artifacts {
        increment(&mut by_kind, format!("{:?}", artifact.kind));
        increment(&mut by_provider, format!("{:?}", artifact.provider));
        increment(&mut by_resolution, format!("{:?}", artifact.resolution));
    }

    let warnings = snapshot
        .warnings
        .iter()
        .map(|warning| {
            json!({
                "id": warning.id,
                "severity": format!("{:?}", warning.severity),
                "title": warning.title,
                "artifactCount": warning.artifact_ids.len(),
            })
        })
        .collect::<Vec<_>>();

    let summary = json!({
        "workspace": snapshot.workspace_path,
        "branch": snapshot.git_branch,
        "scannedAt": snapshot.scanned_at,
        "total": snapshot.artifacts.len(),
        "byKind": by_kind,
        "byProvider": by_provider,
        "byResolution": by_resolution,
        "warnings": warnings,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn increment(counts: &mut BTreeMap<String, usize>, key: String) {
    *counts.entry(key).or_default() += 1;
}
