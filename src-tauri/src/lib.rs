pub mod model;
mod redaction;
pub mod runtime;
pub mod scanner;

use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
    time::SystemTime,
};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use model::HarnessSnapshot;
use runtime::{CodexRunDetail, CodexRuntimeSnapshot};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileIdentity {
    len: u64,
    modified: Option<SystemTime>,
    content_hash: String,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

#[derive(Default)]
struct ScannedArtifactPaths(Mutex<HashMap<PathBuf, FileIdentity>>);

#[derive(Default)]
struct ScannedRuntimeThreads(Mutex<HashSet<String>>);

#[derive(Default)]
struct CurrentWorkspace(Mutex<Option<PathBuf>>);

fn hash_file(path: &Path, expected_len: u64) -> Result<String, String> {
    const MAX_OPENABLE_BYTES: u64 = 16 * 1024 * 1024;
    if expected_len > MAX_OPENABLE_BYTES {
        return Err("Source now exceeds the 16 MiB open limit.".to_string());
    }
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    let mut total = 0_u64;
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        total = total.saturating_add(count as u64);
        if total > MAX_OPENABLE_BYTES {
            return Err("Source changed while verifying and exceeded 16 MiB.".to_string());
        }
        hasher.update(&buffer[..count]);
    }
    if total != expected_len {
        return Err("Source changed while it was being verified.".to_string());
    }
    Ok(hex::encode(hasher.finalize()))
}

fn file_identity(path: &Path, known_hash: Option<String>) -> Result<FileIdentity, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("Resolved path is not a regular file.".to_string());
    }
    Ok(FileIdentity {
        len: metadata.len(),
        modified: metadata.modified().ok(),
        content_hash: match known_hash {
            Some(hash) => hash,
            None => hash_file(path, metadata.len())?,
        },
        #[cfg(unix)]
        device: metadata.dev(),
        #[cfg(unix)]
        inode: metadata.ino(),
    })
}

fn scan_authorized_workspace(
    workspace: PathBuf,
    scanned_paths: &ScannedArtifactPaths,
    runtime_threads: &ScannedRuntimeThreads,
    current_workspace: &CurrentWorkspace,
) -> Result<HarnessSnapshot, String> {
    let workspace = workspace
        .canonicalize()
        .map_err(|error| format!("Unable to open workspace: {error}"))?;
    let home =
        dirs::home_dir().ok_or_else(|| "Unable to locate the user home directory.".to_string())?;
    let snapshot = scanner::scan(&workspace, &home)?;
    let paths = snapshot
        .artifacts
        .iter()
        .filter_map(|artifact| {
            let path = Path::new(&artifact.path).canonicalize().ok()?;
            let identity = file_identity(&path, Some(artifact.content_hash.clone())).ok()?;
            Some((path, identity))
        })
        .collect();
    *scanned_paths
        .0
        .lock()
        .map_err(|_| "Unable to update the scanned artifact allowlist.".to_string())? = paths;
    runtime_threads
        .0
        .lock()
        .map_err(|_| "Unable to reset the runtime thread allowlist.".to_string())?
        .clear();
    *current_workspace
        .0
        .lock()
        .map_err(|_| "Unable to update the authorized workspace.".to_string())? = Some(workspace);
    Ok(snapshot)
}

#[tauri::command]
async fn choose_workspace(
    app: AppHandle,
    title: String,
    scanned_paths: State<'_, ScannedArtifactPaths>,
    runtime_threads: State<'_, ScannedRuntimeThreads>,
    current_workspace: State<'_, CurrentWorkspace>,
) -> Result<Option<HarnessSnapshot>, String> {
    let Some(selection) = app.dialog().file().set_title(title).blocking_pick_folder() else {
        return Ok(None);
    };
    let workspace = selection
        .into_path()
        .map_err(|error| format!("Unable to use the selected workspace: {error}"))?;
    scan_authorized_workspace(
        workspace,
        &scanned_paths,
        &runtime_threads,
        &current_workspace,
    )
    .map(Some)
}

#[tauri::command]
fn load_default_workspace(
    scanned_paths: State<'_, ScannedArtifactPaths>,
    runtime_threads: State<'_, ScannedRuntimeThreads>,
    current_workspace: State<'_, CurrentWorkspace>,
) -> Result<Option<HarnessSnapshot>, String> {
    let Some(workspace) = std::env::var_os("HARNESS_LENS_WORKSPACE").map(PathBuf::from) else {
        return Ok(None);
    };
    scan_authorized_workspace(
        workspace,
        &scanned_paths,
        &runtime_threads,
        &current_workspace,
    )
    .map(Some)
}

#[tauri::command]
fn rescan_workspace(
    scanned_paths: State<'_, ScannedArtifactPaths>,
    runtime_threads: State<'_, ScannedRuntimeThreads>,
    current_workspace: State<'_, CurrentWorkspace>,
) -> Result<HarnessSnapshot, String> {
    let workspace = current_workspace
        .0
        .lock()
        .map_err(|_| "Unable to read the authorized workspace.".to_string())?
        .clone()
        .ok_or_else(|| "Choose a workspace before rescanning.".to_string())?;
    scan_authorized_workspace(
        workspace,
        &scanned_paths,
        &runtime_threads,
        &current_workspace,
    )
}

#[tauri::command]
fn inspect_runtime(
    runtime_threads: State<'_, ScannedRuntimeThreads>,
    current_workspace: State<'_, CurrentWorkspace>,
) -> Result<CodexRuntimeSnapshot, String> {
    let workspace = current_workspace
        .0
        .lock()
        .map_err(|_| "Unable to read the authorized workspace.".to_string())?
        .clone()
        .ok_or_else(|| "Choose a workspace before inspecting runtime evidence.".to_string())?;

    let snapshot = runtime::inspect_workspace(&workspace);
    *runtime_threads
        .0
        .lock()
        .map_err(|_| "Unable to update the runtime thread allowlist.".to_string())? =
        snapshot.runs.iter().map(|run| run.id.clone()).collect();
    Ok(snapshot)
}

#[tauri::command]
fn load_runtime_run(
    thread_id: String,
    runtime_threads: State<'_, ScannedRuntimeThreads>,
) -> Result<CodexRunDetail, String> {
    let allowed = runtime_threads
        .0
        .lock()
        .map_err(|_| "Unable to read the runtime thread allowlist.".to_string())?
        .contains(&thread_id);
    if !allowed {
        return Err("The run is not part of the current runtime snapshot.".to_string());
    }
    runtime::load_run(&thread_id)
}

#[tauri::command]
fn open_artifact(
    app: AppHandle,
    path: String,
    scanned_paths: State<'_, ScannedArtifactPaths>,
) -> Result<(), String> {
    let canonical_path = Path::new(&path)
        .canonicalize()
        .map_err(|error| format!("Unable to open source: {error}"))?;
    let expected_identity = scanned_paths
        .0
        .lock()
        .map_err(|_| "Unable to read the scanned artifact allowlist.".to_string())?
        .get(&canonical_path)
        .cloned()
        .ok_or_else(|| "The source is not part of the current scanned snapshot.".to_string())?;
    let current_identity = file_identity(&canonical_path, None)
        .map_err(|error| format!("Unable to verify source before opening: {error}"))?;
    if current_identity != expected_identity {
        return Err("The source changed after the scan. Rescan before opening it.".to_string());
    }

    app.opener()
        .open_path(canonical_path.to_string_lossy(), None::<String>)
        .map_err(|error| format!("Unable to open source: {error}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ScannedArtifactPaths::default())
        .manage(ScannedRuntimeThreads::default())
        .manage(CurrentWorkspace::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            choose_workspace,
            load_default_workspace,
            rescan_workspace,
            inspect_runtime,
            load_runtime_run,
            open_artifact
        ])
        .run(tauri::generate_context!())
        .expect("error while running Harness Lens");
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::file_identity;

    #[test]
    fn file_identity_detects_same_length_content_replacement() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("AGENTS.md");
        fs::write(&path, b"one").expect("initial fixture");
        let before = file_identity(&path, None).expect("initial identity");

        fs::write(&path, b"two").expect("replacement fixture");
        let after = file_identity(&path, None).expect("replacement identity");

        assert_ne!(before.content_hash, after.content_hash);
        assert_ne!(before, after);
    }
}
