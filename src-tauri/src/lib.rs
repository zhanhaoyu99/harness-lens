pub mod compatibility_report;
mod memory_edit;
pub mod model;
mod redaction;
pub mod runtime;
pub mod scanner;
mod snapshot_store;

use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant, SystemTime},
};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use memory_edit::{
    load_memory_file, memory_editability, replace_memory_file, validate_memory_content,
    verify_memory_revision,
};
use model::{HarnessKind, HarnessSnapshot};
use runtime::{CodexRunDetail, CodexRuntimeSnapshot};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;

const MEMORY_EDIT_SESSION_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FileIdentity {
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

#[derive(Clone, Debug)]
struct MemoryAuthorization {
    artifact_id: String,
    path: PathBuf,
    display_path: String,
    identity: FileIdentity,
    editable: bool,
    editability_reason: Option<String>,
}

#[derive(Default)]
struct ScannedMemoryArtifacts(Mutex<HashMap<String, MemoryAuthorization>>);

#[derive(Clone, Debug, Eq, PartialEq)]
struct MemoryEditSession {
    artifact_id: String,
    path: PathBuf,
    display_path: String,
    expected_identity: FileIdentity,
    issued_at: Instant,
}

#[derive(Default)]
struct MemoryEditSessions(Mutex<HashMap<String, MemoryEditSession>>);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryDocumentDto {
    artifact_id: String,
    edit_token: Option<String>,
    content: String,
    content_hash: String,
    size_bytes: u64,
    editable: bool,
    editability_reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemorySaveResultDto {
    artifact_id: String,
    saved: bool,
    content_hash: String,
    size_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemorySaveErrorDto {
    message: String,
    token_consumed: bool,
}

impl MemorySaveErrorDto {
    fn token_retained(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            token_consumed: false,
        }
    }

    fn token_consumed(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            token_consumed: true,
        }
    }
}

#[derive(Default)]
struct ScannedRuntimeThreads(Mutex<HashSet<String>>);

#[derive(Default)]
struct CurrentWorkspace(Mutex<Option<PathBuf>>);

#[derive(Default)]
struct WorkspaceScanOperations(Mutex<()>);

#[derive(Default)]
struct SnapshotStoreOperations(Mutex<()>);

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

fn safe_dialog_path(path: &str) -> String {
    const MAX_CHARS: usize = 240;
    let sanitized = path
        .chars()
        .map(|character| {
            let bidi_control = matches!(
                character,
                '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
            );
            if character.is_control() || bidi_control {
                '\u{fffd}'
            } else {
                character
            }
        })
        .collect::<Vec<_>>();
    if sanitized.len() <= MAX_CHARS {
        return sanitized.into_iter().collect();
    }
    let head = sanitized.iter().take(130).copied();
    let tail = sanitized
        .iter()
        .skip(sanitized.len().saturating_sub(90))
        .copied();
    head.chain(['…']).chain(tail).collect()
}

fn scan_authorized_workspace(
    workspace: PathBuf,
    scanned_paths: &ScannedArtifactPaths,
    scanned_memories: &ScannedMemoryArtifacts,
    memory_edit_sessions: &MemoryEditSessions,
    runtime_threads: &ScannedRuntimeThreads,
    current_workspace: &CurrentWorkspace,
) -> Result<HarnessSnapshot, String> {
    let workspace = workspace
        .canonicalize()
        .map_err(|error| format!("Unable to open workspace: {error}"))?;
    let home =
        dirs::home_dir().ok_or_else(|| "Unable to locate the user home directory.".to_string())?;
    let snapshot = scanner::scan(&workspace, &home)?;
    let paths: HashMap<PathBuf, FileIdentity> = snapshot
        .artifacts
        .iter()
        .filter_map(|artifact| {
            let path = Path::new(&artifact.path).canonicalize().ok()?;
            let identity = file_identity(&path, Some(artifact.content_hash.clone())).ok()?;
            Some((path, identity))
        })
        .collect();
    let memories = snapshot
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind == HarnessKind::Memory)
        .filter_map(|artifact| {
            let path = Path::new(&artifact.path).canonicalize().ok()?;
            let identity = paths.get(&path)?.clone();
            let (policy_editable, policy_reason) = memory_editability(&path);
            let editable = artifact.editable && policy_editable;
            let editability_reason = artifact.editability_reason.clone().or(policy_reason);
            Some((
                artifact.id.clone(),
                MemoryAuthorization {
                    artifact_id: artifact.id.clone(),
                    path,
                    display_path: safe_dialog_path(&artifact.relative_path),
                    identity,
                    editable,
                    editability_reason,
                },
            ))
        })
        .collect();
    *scanned_paths
        .0
        .lock()
        .map_err(|_| "Unable to update the scanned artifact allowlist.".to_string())? = paths;
    *scanned_memories
        .0
        .lock()
        .map_err(|_| "Unable to update the scanned memory allowlist.".to_string())? = memories;
    memory_edit_sessions
        .0
        .lock()
        .map_err(|_| "Unable to reset memory edit sessions.".to_string())?
        .clear();
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
#[allow(clippy::too_many_arguments)]
async fn choose_workspace(
    app: AppHandle,
    title: String,
    scanned_paths: State<'_, ScannedArtifactPaths>,
    scanned_memories: State<'_, ScannedMemoryArtifacts>,
    memory_edit_sessions: State<'_, MemoryEditSessions>,
    runtime_threads: State<'_, ScannedRuntimeThreads>,
    current_workspace: State<'_, CurrentWorkspace>,
    workspace_operations: State<'_, WorkspaceScanOperations>,
) -> Result<Option<HarnessSnapshot>, String> {
    let Some(selection) = app.dialog().file().set_title(title).blocking_pick_folder() else {
        return Ok(None);
    };
    let workspace = selection
        .into_path()
        .map_err(|error| format!("Unable to use the selected workspace: {error}"))?;
    let _operation = workspace_operations
        .0
        .lock()
        .map_err(|_| "Unable to start the workspace scan.".to_string())?;
    scan_authorized_workspace(
        workspace,
        &scanned_paths,
        &scanned_memories,
        &memory_edit_sessions,
        &runtime_threads,
        &current_workspace,
    )
    .map(Some)
}

#[tauri::command]
fn load_default_workspace(
    scanned_paths: State<'_, ScannedArtifactPaths>,
    scanned_memories: State<'_, ScannedMemoryArtifacts>,
    memory_edit_sessions: State<'_, MemoryEditSessions>,
    runtime_threads: State<'_, ScannedRuntimeThreads>,
    current_workspace: State<'_, CurrentWorkspace>,
    workspace_operations: State<'_, WorkspaceScanOperations>,
) -> Result<Option<HarnessSnapshot>, String> {
    let Some(workspace) = std::env::var_os("HARNESS_LENS_WORKSPACE").map(PathBuf::from) else {
        return Ok(None);
    };
    let _operation = workspace_operations
        .0
        .lock()
        .map_err(|_| "Unable to start the workspace scan.".to_string())?;
    scan_authorized_workspace(
        workspace,
        &scanned_paths,
        &scanned_memories,
        &memory_edit_sessions,
        &runtime_threads,
        &current_workspace,
    )
    .map(Some)
}

#[tauri::command]
fn rescan_workspace(
    scanned_paths: State<'_, ScannedArtifactPaths>,
    scanned_memories: State<'_, ScannedMemoryArtifacts>,
    memory_edit_sessions: State<'_, MemoryEditSessions>,
    runtime_threads: State<'_, ScannedRuntimeThreads>,
    current_workspace: State<'_, CurrentWorkspace>,
    workspace_operations: State<'_, WorkspaceScanOperations>,
) -> Result<HarnessSnapshot, String> {
    let _operation = workspace_operations
        .0
        .lock()
        .map_err(|_| "Unable to start the workspace scan.".to_string())?;
    let workspace = current_workspace
        .0
        .lock()
        .map_err(|_| "Unable to read the authorized workspace.".to_string())?
        .clone()
        .ok_or_else(|| "Choose a workspace before rescanning.".to_string())?;
    scan_authorized_workspace(
        workspace,
        &scanned_paths,
        &scanned_memories,
        &memory_edit_sessions,
        &runtime_threads,
        &current_workspace,
    )
}

fn authorized_workspace(current_workspace: &CurrentWorkspace) -> Result<PathBuf, String> {
    current_workspace
        .0
        .lock()
        .map_err(|_| "Unable to read the authorized workspace.".to_string())?
        .clone()
        .ok_or_else(|| "Choose a workspace before using snapshot history.".to_string())
}

fn snapshot_storage_root(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("Unable to locate local snapshot storage: {error}"))
}

#[tauri::command]
fn list_context_snapshots(
    app: AppHandle,
    current_workspace: State<'_, CurrentWorkspace>,
    workspace_operations: State<'_, WorkspaceScanOperations>,
    store_operations: State<'_, SnapshotStoreOperations>,
) -> Result<Vec<snapshot_store::ContextSnapshotSummary>, String> {
    let _workspace_operation = workspace_operations
        .0
        .lock()
        .map_err(|_| "Unable to authorize reading snapshot history.".to_string())?;
    let workspace = authorized_workspace(&current_workspace)?;
    let _operation = store_operations
        .0
        .lock()
        .map_err(|_| "Unable to read snapshot history.".to_string())?;
    snapshot_store::list(&snapshot_storage_root(&app)?, &workspace)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn capture_context_snapshot(
    app: AppHandle,
    scanned_paths: State<'_, ScannedArtifactPaths>,
    scanned_memories: State<'_, ScannedMemoryArtifacts>,
    memory_edit_sessions: State<'_, MemoryEditSessions>,
    runtime_threads: State<'_, ScannedRuntimeThreads>,
    current_workspace: State<'_, CurrentWorkspace>,
    workspace_operations: State<'_, WorkspaceScanOperations>,
    store_operations: State<'_, SnapshotStoreOperations>,
) -> Result<snapshot_store::ContextSnapshotCaptureResult, String> {
    let _scan_operation = workspace_operations
        .0
        .lock()
        .map_err(|_| "Unable to start the snapshot scan.".to_string())?;
    let workspace = authorized_workspace(&current_workspace)?;
    // A saved capture always starts from a fresh backend scan. The frontend cannot
    // submit or persist its mutable copy of the live snapshot.
    let live_snapshot = scan_authorized_workspace(
        workspace.clone(),
        &scanned_paths,
        &scanned_memories,
        &memory_edit_sessions,
        &runtime_threads,
        &current_workspace,
    )?;
    let _store_operation = store_operations
        .0
        .lock()
        .map_err(|_| "Unable to store snapshot history.".to_string())?;
    let storage_root = snapshot_storage_root(&app)?;
    let result = snapshot_store::capture(&storage_root, &workspace, &live_snapshot);
    Ok(match result {
        Ok(outcome) => snapshot_store::ContextSnapshotCaptureResult {
            live_snapshot,
            captured: Some(outcome.captured),
            history: outcome.history,
            persistence_error: None,
            storage_status: outcome.storage_status,
        },
        Err(error) => snapshot_store::ContextSnapshotCaptureResult {
            live_snapshot,
            captured: None,
            history: snapshot_store::list(&storage_root, &workspace).unwrap_or_default(),
            persistence_error: Some(error),
            storage_status: snapshot_store::ContextSnapshotStorageStatus::default(),
        },
    })
}

#[tauri::command]
fn load_context_snapshot(
    app: AppHandle,
    capture_id: String,
    current_workspace: State<'_, CurrentWorkspace>,
    workspace_operations: State<'_, WorkspaceScanOperations>,
    store_operations: State<'_, SnapshotStoreOperations>,
) -> Result<snapshot_store::StoredContextSnapshot, String> {
    let _workspace_operation = workspace_operations
        .0
        .lock()
        .map_err(|_| "Unable to authorize reading snapshot history.".to_string())?;
    let workspace = authorized_workspace(&current_workspace)?;
    let _operation = store_operations
        .0
        .lock()
        .map_err(|_| "Unable to read snapshot history.".to_string())?;
    snapshot_store::load(&snapshot_storage_root(&app)?, &workspace, &capture_id)
}

#[tauri::command]
fn compare_context_snapshots(
    app: AppHandle,
    base_capture_id: String,
    target_capture_id: String,
    current_workspace: State<'_, CurrentWorkspace>,
    workspace_operations: State<'_, WorkspaceScanOperations>,
    store_operations: State<'_, SnapshotStoreOperations>,
) -> Result<snapshot_store::ContextSnapshotComparison, String> {
    let _workspace_operation = workspace_operations
        .0
        .lock()
        .map_err(|_| "Unable to authorize comparing snapshot history.".to_string())?;
    let workspace = authorized_workspace(&current_workspace)?;
    let _operation = store_operations
        .0
        .lock()
        .map_err(|_| "Unable to compare snapshot history.".to_string())?;
    snapshot_store::compare(
        &snapshot_storage_root(&app)?,
        &workspace,
        &base_capture_id,
        &target_capture_id,
    )
}

#[tauri::command]
async fn clear_context_snapshot_history(
    app: AppHandle,
    current_workspace: State<'_, CurrentWorkspace>,
    store_operations: State<'_, SnapshotStoreOperations>,
    workspace_operations: State<'_, WorkspaceScanOperations>,
) -> Result<snapshot_store::ContextSnapshotClearResult, String> {
    let workspace = authorized_workspace(&current_workspace)?;
    let workspace_name = workspace
        .file_name()
        .and_then(|value| value.to_str())
        .map(safe_dialog_path)
        .unwrap_or_else(|| "workspace".to_string());
    let confirm_app = app.clone();
    let confirmed = tauri::async_runtime::spawn_blocking(move || {
        confirm_app
            .dialog()
            .message(format!(
                "永久清空此工作区的本地快照历史？\n{workspace_name}\n\nPermanently clear local snapshot history for this workspace?\n{workspace_name}"
            ))
            .title("清空快照历史 / Clear snapshot history")
            .buttons(MessageDialogButtons::OkCancelCustom(
                "清空 / Clear".to_string(),
                "取消 / Cancel".to_string(),
            ))
            .blocking_show()
    })
    .await
    .map_err(|error| format!("Unable to show the clear-history confirmation: {error}"))?;
    if !confirmed {
        return Ok(snapshot_store::ContextSnapshotClearResult { cleared: false });
    }
    let _workspace_operation = workspace_operations
        .0
        .lock()
        .map_err(|_| "Unable to authorize clearing snapshot history.".to_string())?;
    let active_workspace = authorized_workspace(&current_workspace)?;
    if active_workspace != workspace {
        return Err("The selected workspace changed before history was cleared.".to_string());
    }
    let _operation = store_operations
        .0
        .lock()
        .map_err(|_| "Unable to clear snapshot history.".to_string())?;
    let active_workspace = authorized_workspace(&current_workspace)?;
    if active_workspace != workspace {
        return Err("The selected workspace changed before history was cleared.".to_string());
    }
    snapshot_store::clear(&snapshot_storage_root(&app)?, &workspace)?;
    Ok(snapshot_store::ContextSnapshotClearResult { cleared: true })
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

fn authorized_memory(
    artifact_id: &str,
    scanned_memories: &ScannedMemoryArtifacts,
) -> Result<MemoryAuthorization, String> {
    scanned_memories
        .0
        .lock()
        .map_err(|_| "Unable to read the scanned memory allowlist.".to_string())?
        .get(artifact_id)
        .cloned()
        .ok_or_else(|| {
            "The item is not a Memory artifact in the current scanned snapshot.".to_string()
        })
}

fn active_memory_edit_session(
    edit_token: &str,
    memory_edit_sessions: &MemoryEditSessions,
) -> Result<MemoryEditSession, (String, bool)> {
    let mut sessions = memory_edit_sessions
        .0
        .lock()
        .map_err(|_| ("Unable to read memory edit sessions.".to_string(), false))?;
    let Some(session) = sessions.get(edit_token).cloned() else {
        return Err((
            "The memory edit session is invalid or has already been used.".to_string(),
            true,
        ));
    };
    if session.issued_at.elapsed() > MEMORY_EDIT_SESSION_TTL {
        sessions.remove(edit_token);
        return Err((
            "The memory edit session expired. Reload the file before saving.".to_string(),
            true,
        ));
    }
    Ok(session)
}

fn consume_memory_edit_session(
    edit_token: &str,
    expected_session: &MemoryEditSession,
    memory_edit_sessions: &MemoryEditSessions,
) -> Result<MemoryEditSession, String> {
    let mut sessions = memory_edit_sessions
        .0
        .lock()
        .map_err(|_| "Unable to update memory edit sessions.".to_string())?;
    let Some(current_session) = sessions.get(edit_token) else {
        return Err("The memory edit session is invalid or has already been used.".to_string());
    };
    if current_session.issued_at.elapsed() > MEMORY_EDIT_SESSION_TTL {
        sessions.remove(edit_token);
        return Err("The memory edit session expired. Reload the file before saving.".to_string());
    }
    if current_session != expected_session {
        return Err("The memory edit session changed. Reload the file before saving.".to_string());
    }
    sessions
        .remove(edit_token)
        .ok_or_else(|| "The memory edit session could not be consumed.".to_string())
}

#[tauri::command]
fn load_memory_artifact(
    artifact_id: String,
    scanned_memories: State<'_, ScannedMemoryArtifacts>,
    memory_edit_sessions: State<'_, MemoryEditSessions>,
) -> Result<MemoryDocumentDto, String> {
    let authorization = authorized_memory(&artifact_id, &scanned_memories)?;
    let loaded = load_memory_file(&authorization.path, &authorization.identity)?;
    let editable = authorization.editable && loaded.safe_to_edit;
    let editability_reason = if editable {
        None
    } else {
        authorization
            .editability_reason
            .clone()
            .or(loaded.unsafe_reason.clone())
            .or_else(|| Some("This memory file is view-only.".to_string()))
    };
    let edit_token = if editable {
        let token = Uuid::new_v4().simple().to_string();
        let session = MemoryEditSession {
            artifact_id: authorization.artifact_id.clone(),
            path: authorization.path,
            display_path: authorization.display_path,
            expected_identity: loaded.identity.clone(),
            issued_at: Instant::now(),
        };
        let mut sessions = memory_edit_sessions
            .0
            .lock()
            .map_err(|_| "Unable to create a memory edit session.".to_string())?;
        sessions.retain(|_, existing| {
            existing.issued_at.elapsed() <= MEMORY_EDIT_SESSION_TTL
                && existing.artifact_id != artifact_id
        });
        sessions.insert(token.clone(), session);
        Some(token)
    } else {
        None
    };

    Ok(MemoryDocumentDto {
        artifact_id,
        edit_token,
        content: loaded.content,
        content_hash: loaded.identity.content_hash,
        size_bytes: loaded.identity.len,
        editable,
        editability_reason,
    })
}

#[tauri::command]
async fn save_memory_artifact(
    edit_token: String,
    content: String,
    app: AppHandle,
    scanned_paths: State<'_, ScannedArtifactPaths>,
    scanned_memories: State<'_, ScannedMemoryArtifacts>,
    memory_edit_sessions: State<'_, MemoryEditSessions>,
) -> Result<MemorySaveResultDto, MemorySaveErrorDto> {
    let session = active_memory_edit_session(&edit_token, &memory_edit_sessions).map_err(
        |(message, token_consumed)| {
            if token_consumed {
                MemorySaveErrorDto::token_consumed(message)
            } else {
                MemorySaveErrorDto::token_retained(message)
            }
        },
    )?;
    validate_memory_content(&content).map_err(MemorySaveErrorDto::token_retained)?;

    let authorization = authorized_memory(&session.artifact_id, &scanned_memories)
        .map_err(MemorySaveErrorDto::token_retained)?;
    if authorization.path != session.path || authorization.identity != session.expected_identity {
        return Err(MemorySaveErrorDto::token_retained(
            "The scanned memory authorization changed. Reload before saving.",
        ));
    }
    if !authorization.editable {
        return Err(MemorySaveErrorDto::token_retained(
            authorization
                .editability_reason
                .unwrap_or_else(|| "This memory file is view-only.".to_string()),
        ));
    }
    verify_memory_revision(&session.path, &session.expected_identity)
        .map_err(MemorySaveErrorDto::token_retained)?;

    let display_path = session.display_path.clone();
    let confirm_app = app.clone();
    let confirmed = tauri::async_runtime::spawn_blocking(move || {
        confirm_app
            .dialog()
            .message(format!(
                "保存对这个记忆文件的修改？\n{display_path}\n\nSave changes to this Memory file?\n{display_path}"
            ))
            .title("保存记忆修改 / Save memory changes")
            .buttons(MessageDialogButtons::OkCancelCustom(
                "保存 / Save".to_string(),
                "取消 / Cancel".to_string(),
            ))
            .blocking_show()
    })
    .await
    .map_err(|error| {
        MemorySaveErrorDto::token_retained(format!(
            "Unable to show the save confirmation: {error}"
        ))
    })?;
    if !confirmed {
        return Ok(MemorySaveResultDto {
            artifact_id: session.artifact_id.clone(),
            saved: false,
            content_hash: session.expected_identity.content_hash.clone(),
            size_bytes: session.expected_identity.len,
        });
    }

    // Revalidate the allowlist and on-disk revision after the user confirmation.
    let authorization = authorized_memory(&session.artifact_id, &scanned_memories)
        .map_err(MemorySaveErrorDto::token_retained)?;
    if authorization.path != session.path || authorization.identity != session.expected_identity {
        return Err(MemorySaveErrorDto::token_retained(
            "The scanned memory authorization changed. Reload before saving.",
        ));
    }
    verify_memory_revision(&session.path, &session.expected_identity)
        .map_err(MemorySaveErrorDto::token_retained)?;

    // Consuming the token is the write boundary. A cancelled confirmation leaves it intact;
    // after this point every outcome requires a reload before another write attempt.
    let session = consume_memory_edit_session(&edit_token, &session, &memory_edit_sessions)
        .map_err(MemorySaveErrorDto::token_consumed)?;

    // Resolve every fallible bookkeeping step before the atomic replacement. Keeping both
    // guards prevents a concurrent rescan and makes the allowlist updates infallible after a
    // successful commit, so an on-disk save is never reported as a failure.
    let mut scanned_paths = scanned_paths.0.lock().map_err(|_| {
        MemorySaveErrorDto::token_consumed("Unable to update the scanned artifact allowlist.")
    })?;
    let mut scanned_memories = scanned_memories.0.lock().map_err(|_| {
        MemorySaveErrorDto::token_consumed("Unable to update the scanned memory allowlist.")
    })?;
    let scanned_path_identity = scanned_paths.get(&session.path).ok_or_else(|| {
        MemorySaveErrorDto::token_consumed("The memory path is no longer in the current scan.")
    })?;
    if scanned_path_identity != &session.expected_identity {
        return Err(MemorySaveErrorDto::token_consumed(
            "The scanned memory authorization changed. Reload before saving.",
        ));
    }
    let saved_authorization = scanned_memories
        .get_mut(&session.artifact_id)
        .ok_or_else(|| {
            MemorySaveErrorDto::token_consumed(
                "The memory authorization changed. Reload before saving.",
            )
        })?;
    if saved_authorization.path != session.path
        || saved_authorization.identity != session.expected_identity
    {
        return Err(MemorySaveErrorDto::token_consumed(
            "The scanned memory authorization changed. Reload before saving.",
        ));
    }

    let saved_identity = replace_memory_file(&session.path, &session.expected_identity, &content)
        .map_err(MemorySaveErrorDto::token_consumed)?;
    scanned_paths.insert(session.path.clone(), saved_identity.clone());
    saved_authorization.identity = saved_identity.clone();

    Ok(MemorySaveResultDto {
        artifact_id: session.artifact_id,
        saved: true,
        content_hash: saved_identity.content_hash,
        size_bytes: saved_identity.len,
    })
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
        .manage(ScannedMemoryArtifacts::default())
        .manage(MemoryEditSessions::default())
        .manage(ScannedRuntimeThreads::default())
        .manage(CurrentWorkspace::default())
        .manage(WorkspaceScanOperations::default())
        .manage(SnapshotStoreOperations::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            choose_workspace,
            load_default_workspace,
            rescan_workspace,
            list_context_snapshots,
            capture_context_snapshot,
            load_context_snapshot,
            compare_context_snapshots,
            clear_context_snapshot_history,
            inspect_runtime,
            load_runtime_run,
            load_memory_artifact,
            save_memory_artifact,
            open_artifact
        ])
        .run(tauri::generate_context!())
        .expect("error while running Harness Lens");
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::Instant};

    use super::{
        active_memory_edit_session, authorized_memory, consume_memory_edit_session, file_identity,
        safe_dialog_path, FileIdentity, MemoryEditSession, MemoryEditSessions,
        ScannedMemoryArtifacts,
    };

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

    #[test]
    fn memory_load_rejects_artifacts_outside_the_current_scan() {
        let scanned_memories = ScannedMemoryArtifacts::default();

        assert!(authorized_memory("not-scanned", &scanned_memories)
            .expect_err("unscanned artifact must be rejected")
            .contains("current scanned snapshot"));
    }

    #[test]
    fn dialog_paths_strip_control_characters_and_bound_length() {
        let unsafe_path = format!("./unsafe\n\u{202e}{}.md", "x".repeat(300));

        let safe_path = safe_dialog_path(&unsafe_path);

        assert!(!safe_path.contains('\n'));
        assert!(!safe_path.contains('\u{202e}'));
        assert!(safe_path.contains('…'));
        assert!(safe_path.chars().count() <= 221);
    }

    #[test]
    fn confirmation_cancel_keeps_the_edit_token_available_for_retry() {
        let sessions = MemoryEditSessions::default();
        let token = "retry-token";
        let session = memory_session_fixture();
        sessions
            .0
            .lock()
            .expect("edit sessions")
            .insert(token.to_string(), session.clone());

        let confirmation_session =
            active_memory_edit_session(token, &sessions).expect("confirmation lookup");
        assert_eq!(confirmation_session, session);

        // A cancelled dialog deliberately does not call consume_memory_edit_session.
        let retry_session =
            active_memory_edit_session(token, &sessions).expect("retry lookup after cancel");
        assert_eq!(retry_session, session);
    }

    #[test]
    fn missing_edit_token_requires_a_reload() {
        let sessions = MemoryEditSessions::default();

        let (message, token_consumed) = active_memory_edit_session("missing", &sessions)
            .expect_err("missing token must be rejected");

        assert!(message.contains("invalid"));
        assert!(token_consumed);
    }

    #[test]
    fn confirmed_write_consumes_the_edit_token_once() {
        let sessions = MemoryEditSessions::default();
        let token = "single-use-token";
        let session = memory_session_fixture();
        sessions
            .0
            .lock()
            .expect("edit sessions")
            .insert(token.to_string(), session.clone());

        let consumed =
            consume_memory_edit_session(token, &session, &sessions).expect("first confirmed write");
        assert_eq!(consumed, session);
        assert!(consume_memory_edit_session(token, &session, &sessions)
            .expect_err("second confirmed write must fail")
            .contains("already been used"));
    }

    #[test]
    fn memory_save_errors_serialize_token_consumption_state() {
        let retained = super::MemorySaveErrorDto::token_retained("retryable");
        let consumed = super::MemorySaveErrorDto::token_consumed("reload required");

        assert_eq!(
            serde_json::to_value(retained).expect("retained error JSON"),
            serde_json::json!({
                "message": "retryable",
                "tokenConsumed": false
            })
        );
        assert_eq!(
            serde_json::to_value(consumed).expect("consumed error JSON"),
            serde_json::json!({
                "message": "reload required",
                "tokenConsumed": true
            })
        );
    }

    fn memory_session_fixture() -> MemoryEditSession {
        MemoryEditSession {
            artifact_id: "memory-artifact".to_string(),
            path: PathBuf::from("/tmp/MEMORY.md"),
            display_path: "./MEMORY.md".to_string(),
            expected_identity: FileIdentity {
                len: 6,
                modified: None,
                content_hash: "fixture-hash".to_string(),
                #[cfg(unix)]
                device: 1,
                #[cfg(unix)]
                inode: 2,
            },
            issued_at: Instant::now(),
        }
    }
}
