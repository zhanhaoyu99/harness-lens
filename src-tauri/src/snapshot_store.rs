use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use chrono::{SecondsFormat, Utc};
use fs2::FileExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use uuid::Uuid;

use crate::model::{
    HarnessArtifact, HarnessKind, HarnessProvider, HarnessScope, HarnessSnapshot, ResolutionState,
    WarningSeverity,
};

const SCHEMA_VERSION: u32 = 1;
const SCANNER_VERSION: &str = "1";
const MAX_CAPTURES_PER_WORKSPACE: usize = 50;
const MAX_INDEX_BYTES: u64 = 256 * 1024;
const MAX_OBJECT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_ITEMS: usize = 4_096;
const MAX_DIAGNOSTICS: usize = 128;
const ARTIFACT_ID_LENGTH: usize = 24;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSnapshotSummary {
    pub capture_id: String,
    pub snapshot_id: String,
    pub schema_version: u32,
    pub workspace_key: String,
    pub workspace_name: String,
    pub git_branch: Option<String>,
    pub captured_at: String,
    pub item_count: usize,
    pub diagnostic_count: usize,
    pub complete: bool,
    pub app_version: String,
    pub scanner_version: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextSnapshotItem {
    pub id: String,
    pub name: String,
    pub kind: HarnessKind,
    pub provider: HarnessProvider,
    pub scope: HarnessScope,
    pub source_label: String,
    pub content_hash: String,
    pub size_bytes: u64,
    pub resolution: ResolutionState,
    pub duplicate_group_id: Option<String>,
    pub counterpart_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextSnapshotDiagnostic {
    pub id: String,
    pub severity: WarningSeverity,
    pub artifact_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredContextSnapshot {
    pub summary: ContextSnapshotSummary,
    pub items: Vec<ContextSnapshotItem>,
    pub diagnostics: Vec<ContextSnapshotDiagnostic>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSnapshotCaptureResult {
    pub live_snapshot: HarnessSnapshot,
    pub captured: Option<ContextSnapshotSummary>,
    pub history: Vec<ContextSnapshotSummary>,
    pub persistence_error: Option<String>,
    pub storage_status: ContextSnapshotStorageStatus,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSnapshotStorageStatus {
    pub cleanup_pending: bool,
    pub cleanup_warning: Option<String>,
    pub durability_warning: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSnapshotClearResult {
    pub cleared: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotArtifactChange {
    pub artifact_id: String,
    pub kind: SnapshotChangeKind,
    pub before: Option<ContextSnapshotItem>,
    pub after: Option<ContextSnapshotItem>,
    pub content_changed: bool,
    pub resolution_changed: bool,
    pub metadata_changed: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SnapshotChangeKind {
    Added,
    Removed,
    Changed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSnapshotComparison {
    pub base: ContextSnapshotSummary,
    pub target: ContextSnapshotSummary,
    pub changes: Vec<SnapshotArtifactChange>,
    pub unchanged_count: usize,
    pub diagnostics_changed: bool,
    pub complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SnapshotPayload {
    schema_version: u32,
    workspace_key: String,
    complete: bool,
    items: Vec<ContextSnapshotItem>,
    diagnostics: Vec<ContextSnapshotDiagnostic>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SnapshotObject {
    schema_version: u32,
    snapshot_id: String,
    payload: SnapshotPayload,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CaptureRecord {
    schema_version: u32,
    capture_id: String,
    snapshot_id: String,
    workspace_key: String,
    workspace_name: String,
    git_branch: Option<String>,
    app_version: String,
    scanner_version: String,
    captured_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkspaceIndex {
    schema_version: u32,
    workspace_key: String,
    captures: Vec<CaptureRecord>,
}

#[derive(Debug)]
pub struct CaptureOutcome {
    pub captured: ContextSnapshotSummary,
    pub history: Vec<ContextSnapshotSummary>,
    pub storage_status: ContextSnapshotStorageStatus,
}

#[derive(Debug)]
struct StoreLock {
    _file: File,
}

impl Drop for StoreLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self._file);
    }
}

#[derive(Debug, Default)]
struct AtomicWriteOutcome {
    durability_warning: Option<String>,
}

pub fn workspace_key(workspace: &Path) -> Result<String, String> {
    let canonical = workspace
        .canonicalize()
        .map_err(|error| format!("Unable to resolve the snapshot workspace: {error}"))?;
    Ok(hex::encode(Sha256::digest(
        canonical.to_string_lossy().as_bytes(),
    )))
}

pub fn list(app_data_root: &Path, workspace: &Path) -> Result<Vec<ContextSnapshotSummary>, String> {
    let key = workspace_key(workspace)?;
    let _lock = acquire_store_lock(app_data_root, &key)?;
    list_locked(app_data_root, &key)
}

fn list_locked(
    app_data_root: &Path,
    workspace_key: &str,
) -> Result<Vec<ContextSnapshotSummary>, String> {
    let root = workspace_store_root(app_data_root, workspace_key);
    validate_existing_store_layout(&root)?;
    let index = load_index(&root, workspace_key)?;
    let history = index
        .captures
        .iter()
        .map(|record| load_summary(&root, record, workspace_key))
        .collect::<Result<Vec<_>, _>>()?;
    // Every successful store access retries cleanup left pending by a prior
    // committed capture. Read operations stay successful if best-effort cleanup
    // is still unavailable.
    let _ = garbage_collect_objects(&root, &index);
    Ok(history)
}

pub fn capture(
    app_data_root: &Path,
    workspace: &Path,
    snapshot: &HarnessSnapshot,
) -> Result<CaptureOutcome, String> {
    let key = workspace_key(workspace)?;
    verify_live_snapshot_workspace(workspace, snapshot)?;
    let _lock = acquire_store_lock(app_data_root, &key)?;
    let root = workspace_store_root(app_data_root, &key);
    ensure_store_layout(app_data_root, &key)?;

    let mut index = load_index(&root, &key)?;
    // Validate retained state before creating a new immutable object. This
    // ensures corrupt history cannot strand a new, unreferenced object.
    let validated_history = index
        .captures
        .iter()
        .map(|item| load_summary(&root, item, &key))
        .collect::<Result<Vec<_>, _>>()?;
    let _ = garbage_collect_objects(&root, &index);

    let payload = project_snapshot(snapshot, key.clone())?;
    let snapshot_id = payload_id(&payload)?;
    let object = SnapshotObject {
        schema_version: SCHEMA_VERSION,
        snapshot_id: snapshot_id.clone(),
        payload,
    };
    validate_object(&object, &snapshot_id, &key)?;
    // Normalize and validate every fallible piece of capture metadata before
    // creating the immutable object. Invalid display metadata must not leave an
    // unreachable object behind.
    let record = CaptureRecord {
        schema_version: SCHEMA_VERSION,
        capture_id: unique_capture_id(&index),
        snapshot_id: snapshot_id.clone(),
        workspace_key: key.clone(),
        workspace_name: redacted_bounded_text(&snapshot.workspace_name, 160, "workspace name")?,
        git_branch: snapshot
            .git_branch
            .as_deref()
            .map(|branch| redacted_bounded_text(branch, 240, "Git branch"))
            .transpose()?,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        scanner_version: SCANNER_VERSION.to_string(),
        captured_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
    };
    validate_record(&record, &key)?;
    let object_path = object_path(&root, &snapshot_id)?;
    let mut object_created = false;
    let mut durability_warnings = Vec::new();
    if path_entry_exists(&object_path)? {
        let existing = load_object(&root, &snapshot_id, &key)?;
        if existing.payload != object.payload {
            return Err(
                "The existing content-addressed snapshot does not match its identifier."
                    .to_string(),
            );
        }
    } else {
        let outcome = atomic_write_json(&object_path, &object, MAX_OBJECT_BYTES)?;
        object_created = true;
        if let Some(warning) = outcome.durability_warning {
            durability_warnings.push(format!("Snapshot object: {warning}"));
        }
    }

    index.captures.insert(0, record.clone());
    index.captures.truncate(MAX_CAPTURES_PER_WORKSPACE);
    let index_outcome = match atomic_write_json(&root.join("index.json"), &index, MAX_INDEX_BYTES) {
        Ok(outcome) => outcome,
        Err(error) => {
            if object_created {
                remove_uncommitted_object(&object_path).map_err(|cleanup_error| {
                    format!("{error} The uncommitted snapshot object could not be removed: {cleanup_error}")
                })?;
            }
            return Err(error);
        }
    };
    if let Some(warning) = index_outcome.durability_warning {
        durability_warnings.push(format!("Snapshot index: {warning}"));
    }
    // The index replacement is the capture commit point. Cleanup is best effort:
    // a stale, unreferenced immutable object is preferable to reporting failure
    // after the new capture is already durable.
    let cleanup_warning = garbage_collect_objects(&root, &index).err();

    // Do not perform fallible storage reads after the index commit point. The
    // new object and retained history were already validated above, so a
    // committed capture is always reported as success (with warnings when
    // durability or cleanup could not be fully confirmed).
    let captured = summary(&record, &object.payload);
    let history = std::iter::once(captured.clone())
        .chain(
            validated_history
                .into_iter()
                .take(MAX_CAPTURES_PER_WORKSPACE.saturating_sub(1)),
        )
        .collect();
    Ok(CaptureOutcome {
        captured,
        history,
        storage_status: ContextSnapshotStorageStatus {
            cleanup_pending: cleanup_warning.is_some(),
            cleanup_warning,
            durability_warning: (!durability_warnings.is_empty())
                .then(|| durability_warnings.join(" ")),
        },
    })
}

pub fn load(
    app_data_root: &Path,
    workspace: &Path,
    capture_id: &str,
) -> Result<StoredContextSnapshot, String> {
    validate_hex_identifier(capture_id, 32, "capture")?;
    let key = workspace_key(workspace)?;
    let _lock = acquire_store_lock(app_data_root, &key)?;
    load_locked(app_data_root, &key, capture_id)
}

fn load_locked(
    app_data_root: &Path,
    workspace_key: &str,
    capture_id: &str,
) -> Result<StoredContextSnapshot, String> {
    let root = workspace_store_root(app_data_root, workspace_key);
    validate_existing_store_layout(&root)?;
    let index = load_index(&root, workspace_key)?;
    let record = index
        .captures
        .iter()
        .find(|record| record.capture_id == capture_id)
        .ok_or_else(|| "The capture is not in the current workspace history.".to_string())?;
    let object = load_object(&root, &record.snapshot_id, workspace_key)?;
    let stored = stored_snapshot(record, object);
    let _ = garbage_collect_objects(&root, &index);
    Ok(stored)
}

pub fn compare(
    app_data_root: &Path,
    workspace: &Path,
    base_capture_id: &str,
    target_capture_id: &str,
) -> Result<ContextSnapshotComparison, String> {
    if base_capture_id == target_capture_id {
        return Err("Choose two different saved captures to compare.".to_string());
    }
    validate_hex_identifier(base_capture_id, 32, "base capture")?;
    validate_hex_identifier(target_capture_id, 32, "target capture")?;
    let key = workspace_key(workspace)?;
    let _lock = acquire_store_lock(app_data_root, &key)?;
    let base = load_locked(app_data_root, &key, base_capture_id)?;
    let target = load_locked(app_data_root, &key, target_capture_id)?;
    if base.summary.workspace_key != target.summary.workspace_key {
        return Err("Saved captures from different workspaces cannot be compared.".to_string());
    }

    let before_by_id = base
        .items
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect::<HashMap<_, _>>();
    let after_by_id = target
        .items
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect::<HashMap<_, _>>();
    let mut changes = Vec::new();
    let mut unchanged_count = 0;

    for before in &base.items {
        let Some(after) = after_by_id.get(before.id.as_str()) else {
            changes.push(change(
                SnapshotChangeKind::Removed,
                Some(before.clone()),
                None,
            ));
            continue;
        };
        let content_changed = before.content_hash != after.content_hash;
        let resolution_changed = before.resolution != after.resolution;
        let metadata_changed = item_metadata_changed(before, after);
        if content_changed || resolution_changed || metadata_changed {
            changes.push(SnapshotArtifactChange {
                artifact_id: before.id.clone(),
                kind: SnapshotChangeKind::Changed,
                before: Some(before.clone()),
                after: Some((*after).clone()),
                content_changed,
                resolution_changed,
                metadata_changed,
            });
        } else {
            unchanged_count += 1;
        }
    }
    for after in &target.items {
        if !before_by_id.contains_key(after.id.as_str()) {
            changes.push(change(SnapshotChangeKind::Added, None, Some(after.clone())));
        }
    }
    changes.sort_by(|left, right| {
        change_order(&left.kind)
            .cmp(&change_order(&right.kind))
            .then_with(|| change_name(left).cmp(change_name(right)))
            .then_with(|| left.artifact_id.cmp(&right.artifact_id))
    });

    Ok(ContextSnapshotComparison {
        base: base.summary.clone(),
        target: target.summary.clone(),
        changes,
        unchanged_count,
        diagnostics_changed: base.diagnostics != target.diagnostics,
        complete: base.summary.complete && target.summary.complete,
    })
}

pub fn clear(app_data_root: &Path, workspace: &Path) -> Result<(), String> {
    let key = workspace_key(workspace)?;
    let _lock = acquire_store_lock(app_data_root, &key)?;
    let root = workspace_store_root(app_data_root, &key);
    if !path_entry_exists(&root)? {
        return Ok(());
    }
    validate_existing_store_layout(&root)?;
    fs::remove_dir_all(&root).map_err(|error| format!("Unable to clear snapshot history: {error}"))
}

fn project_snapshot(
    snapshot: &HarnessSnapshot,
    workspace_key: String,
) -> Result<SnapshotPayload, String> {
    if snapshot.artifacts.len() > MAX_ITEMS {
        return Err(format!(
            "The scan contains more than {MAX_ITEMS} items and cannot be captured."
        ));
    }
    if snapshot.warnings.len() > MAX_DIAGNOSTICS {
        return Err(format!(
            "The scan contains more than {MAX_DIAGNOSTICS} diagnostics and cannot be captured."
        ));
    }
    let mut items = snapshot
        .artifacts
        .iter()
        .map(project_item)
        .collect::<Result<Vec<_>, _>>()?;
    items.sort_by(|left, right| left.id.cmp(&right.id));
    let mut diagnostics = snapshot
        .warnings
        .iter()
        .map(|warning| {
            let mut artifact_ids = warning.artifact_ids.clone();
            artifact_ids.sort();
            artifact_ids.dedup();
            Ok(ContextSnapshotDiagnostic {
                id: normalized_diagnostic_id(&warning.id),
                severity: warning.severity.clone(),
                artifact_ids,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    diagnostics.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| severity_order(&left.severity).cmp(&severity_order(&right.severity)))
    });

    Ok(SnapshotPayload {
        schema_version: SCHEMA_VERSION,
        workspace_key,
        complete: !snapshot.warnings.iter().any(|warning| {
            warning.id == "scan-incomplete" || warning.severity == WarningSeverity::Error
        }),
        items,
        diagnostics,
    })
}

fn normalized_diagnostic_id(id: &str) -> String {
    let category = if id == "scan-incomplete" {
        "scan-incomplete"
    } else if id == "runtime-not-connected" {
        "runtime-not-connected"
    } else if id.starts_with("duplicate:") {
        "duplicate"
    } else if id.starts_with("counterpart-difference:") {
        "counterpart-difference"
    } else {
        "diagnostic"
    };
    format!(
        "{category}:{}",
        &hex::encode(Sha256::digest(id.as_bytes()))[..16]
    )
}

fn project_item(artifact: &HarnessArtifact) -> Result<ContextSnapshotItem, String> {
    validate_hex_identifier(&artifact.id, ARTIFACT_ID_LENGTH, "artifact")?;
    validate_hex_identifier(&artifact.content_hash, 64, "content hash")?;
    Ok(ContextSnapshotItem {
        id: artifact.id.clone(),
        name: persistent_artifact_name(artifact)?,
        kind: artifact.kind.clone(),
        provider: artifact.provider.clone(),
        scope: artifact.scope.clone(),
        source_label: format!(
            "{} · {}",
            provider_label(&artifact.provider),
            scope_label(&artifact.scope)
        ),
        content_hash: artifact.content_hash.clone(),
        size_bytes: artifact.size_bytes,
        resolution: artifact.resolution.clone(),
        duplicate_group_id: artifact
            .duplicate_group_id
            .as_deref()
            .map(|value| bounded_text(value, 128, "duplicate group"))
            .transpose()?,
        counterpart_id: artifact
            .counterpart_id
            .as_deref()
            .map(|value| {
                validate_hex_identifier(value, ARTIFACT_ID_LENGTH, "counterpart artifact")?;
                Ok::<_, String>(value.to_string())
            })
            .transpose()?,
    })
}

fn payload_id(payload: &SnapshotPayload) -> Result<String, String> {
    let bytes = serde_json::to_vec(payload)
        .map_err(|error| format!("Unable to normalize the context snapshot: {error}"))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn load_index(root: &Path, workspace_key: &str) -> Result<WorkspaceIndex, String> {
    let path = root.join("index.json");
    if !path_entry_exists(&path)? {
        return Ok(WorkspaceIndex {
            schema_version: SCHEMA_VERSION,
            workspace_key: workspace_key.to_string(),
            captures: Vec::new(),
        });
    }
    let index: WorkspaceIndex = read_json(&path, MAX_INDEX_BYTES, "snapshot history index")?;
    if index.schema_version != SCHEMA_VERSION {
        return Err("The snapshot history uses an unsupported schema version.".to_string());
    }
    if index.workspace_key != workspace_key {
        return Err("The snapshot history belongs to a different workspace.".to_string());
    }
    if index.captures.len() > MAX_CAPTURES_PER_WORKSPACE {
        return Err("The snapshot history exceeds its capture limit.".to_string());
    }
    let mut seen = HashSet::new();
    for record in &index.captures {
        validate_record(record, workspace_key)?;
        if !seen.insert(record.capture_id.as_str()) {
            return Err("The snapshot history contains duplicate capture identifiers.".to_string());
        }
    }
    Ok(index)
}

fn validate_record(record: &CaptureRecord, workspace_key: &str) -> Result<(), String> {
    if record.schema_version != SCHEMA_VERSION || record.workspace_key != workspace_key {
        return Err("The snapshot capture record has incompatible metadata.".to_string());
    }
    validate_hex_identifier(&record.capture_id, 32, "capture")?;
    validate_hex_identifier(&record.snapshot_id, 64, "snapshot")?;
    bounded_text(&record.workspace_name, 160, "workspace name")?;
    if let Some(branch) = &record.git_branch {
        bounded_text(branch, 240, "Git branch")?;
    }
    bounded_text(&record.app_version, 64, "app version")?;
    bounded_text(&record.scanner_version, 64, "scanner version")?;
    chrono::DateTime::parse_from_rfc3339(&record.captured_at)
        .map_err(|_| "The snapshot capture timestamp is invalid.".to_string())?;
    Ok(())
}

fn unique_capture_id(index: &WorkspaceIndex) -> String {
    loop {
        let candidate = Uuid::new_v4().simple().to_string();
        if index
            .captures
            .iter()
            .all(|record| record.capture_id != candidate)
        {
            return candidate;
        }
    }
}

fn load_object(
    root: &Path,
    snapshot_id: &str,
    workspace_key: &str,
) -> Result<SnapshotObject, String> {
    validate_hex_identifier(snapshot_id, 64, "snapshot")?;
    let path = object_path(root, snapshot_id)?;
    let object: SnapshotObject = read_json(&path, MAX_OBJECT_BYTES, "context snapshot")?;
    validate_object(&object, snapshot_id, workspace_key)?;
    Ok(object)
}

fn validate_object(
    object: &SnapshotObject,
    expected_id: &str,
    workspace_key: &str,
) -> Result<(), String> {
    if object.schema_version != SCHEMA_VERSION || object.payload.schema_version != SCHEMA_VERSION {
        return Err("The context snapshot uses an unsupported schema version.".to_string());
    }
    if object.snapshot_id != expected_id || object.payload.workspace_key != workspace_key {
        return Err(
            "The context snapshot identity does not match the current workspace.".to_string(),
        );
    }
    validate_hex_identifier(&object.payload.workspace_key, 64, "workspace")?;
    if object.payload.items.len() > MAX_ITEMS || object.payload.diagnostics.len() > MAX_DIAGNOSTICS
    {
        return Err("The context snapshot exceeds its schema limits.".to_string());
    }
    let item_ids = object
        .payload
        .items
        .iter()
        .map(|item| item.id.as_str())
        .collect::<HashSet<_>>();
    let mut previous_item_id: Option<&str> = None;
    for item in &object.payload.items {
        validate_hex_identifier(&item.id, ARTIFACT_ID_LENGTH, "artifact")?;
        validate_hex_identifier(&item.content_hash, 64, "content hash")?;
        bounded_text(&item.name, 240, "artifact name")?;
        bounded_text(&item.source_label, 80, "source label")?;
        if previous_item_id.is_some_and(|previous| previous >= item.id.as_str()) {
            return Err("The context snapshot items are not uniquely sorted.".to_string());
        }
        previous_item_id = Some(&item.id);
        if let Some(group) = &item.duplicate_group_id {
            bounded_text(group, 128, "duplicate group")?;
        }
        if let Some(counterpart_id) = &item.counterpart_id {
            validate_hex_identifier(counterpart_id, ARTIFACT_ID_LENGTH, "counterpart artifact")?;
            if !item_ids.contains(counterpart_id.as_str()) {
                return Err("The context snapshot references an unknown counterpart.".to_string());
            }
        }
    }
    let mut previous_diagnostic_key: Option<(String, u8)> = None;
    for diagnostic in &object.payload.diagnostics {
        bounded_text(&diagnostic.id, 240, "diagnostic identifier")?;
        let key = (diagnostic.id.clone(), severity_order(&diagnostic.severity));
        if previous_diagnostic_key
            .as_ref()
            .is_some_and(|previous| previous >= &key)
        {
            return Err("The context snapshot diagnostics are not uniquely sorted.".to_string());
        }
        previous_diagnostic_key = Some(key);
        let mut previous_artifact_id: Option<&str> = None;
        for artifact_id in &diagnostic.artifact_ids {
            validate_hex_identifier(artifact_id, ARTIFACT_ID_LENGTH, "diagnostic artifact")?;
            if !item_ids.contains(artifact_id.as_str()) {
                return Err(
                    "The context snapshot diagnostic references an unknown item.".to_string(),
                );
            }
            if previous_artifact_id.is_some_and(|previous| previous >= artifact_id.as_str()) {
                return Err("The diagnostic item identifiers are not uniquely sorted.".to_string());
            }
            previous_artifact_id = Some(artifact_id);
        }
    }
    let actual_id = payload_id(&object.payload)?;
    if actual_id != expected_id {
        return Err("The content-addressed snapshot failed its integrity check.".to_string());
    }
    Ok(())
}

fn load_summary(
    root: &Path,
    record: &CaptureRecord,
    workspace_key: &str,
) -> Result<ContextSnapshotSummary, String> {
    validate_record(record, workspace_key)?;
    let object = load_object(root, &record.snapshot_id, workspace_key)?;
    Ok(summary(record, &object.payload))
}

fn stored_snapshot(record: &CaptureRecord, object: SnapshotObject) -> StoredContextSnapshot {
    StoredContextSnapshot {
        summary: summary(record, &object.payload),
        items: object.payload.items,
        diagnostics: object.payload.diagnostics,
    }
}

fn summary(record: &CaptureRecord, payload: &SnapshotPayload) -> ContextSnapshotSummary {
    ContextSnapshotSummary {
        capture_id: record.capture_id.clone(),
        snapshot_id: record.snapshot_id.clone(),
        schema_version: payload.schema_version,
        workspace_key: payload.workspace_key.clone(),
        workspace_name: record.workspace_name.clone(),
        git_branch: record.git_branch.clone(),
        captured_at: record.captured_at.clone(),
        item_count: payload.items.len(),
        diagnostic_count: payload.diagnostics.len(),
        complete: payload.complete,
        app_version: record.app_version.clone(),
        scanner_version: record.scanner_version.clone(),
    }
}

fn verify_live_snapshot_workspace(
    workspace: &Path,
    snapshot: &HarnessSnapshot,
) -> Result<(), String> {
    let expected = workspace
        .canonicalize()
        .map_err(|error| format!("Unable to resolve the current workspace: {error}"))?;
    let actual = Path::new(&snapshot.workspace_path)
        .canonicalize()
        .map_err(|error| format!("Unable to verify the fresh scan workspace: {error}"))?;
    if actual != expected {
        return Err("The fresh scan does not match the authorized workspace.".to_string());
    }
    Ok(())
}

fn workspace_store_root(app_data_root: &Path, workspace_key: &str) -> PathBuf {
    app_data_root
        .join("context-snapshots")
        .join("v1")
        .join("workspaces")
        .join(workspace_key)
}

fn acquire_store_lock(app_data_root: &Path, workspace_key: &str) -> Result<StoreLock, String> {
    validate_hex_identifier(workspace_key, 64, "workspace")?;
    let version_root = ensure_snapshot_version_root(app_data_root)?;
    let locks = version_root.join("locks");
    ensure_regular_directory(&locks, "snapshot lock directory")?;
    let path = locks.join(format!("{workspace_key}.lock"));
    validate_optional_regular_file(&path, "snapshot store lock")?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|error| format!("Unable to open the snapshot store lock: {error}"))?;
    validate_regular_file(&path, "snapshot store lock")?;
    file.lock_exclusive()
        .map_err(|error| format!("Unable to lock snapshot storage: {error}"))?;
    // Recheck after acquiring the advisory lock. This also rejects replacement
    // with a symlink while this process was waiting for another app instance.
    validate_regular_file(&path, "snapshot store lock")?;
    Ok(StoreLock { _file: file })
}

fn ensure_snapshot_version_root(app_data_root: &Path) -> Result<PathBuf, String> {
    ensure_regular_directory(app_data_root, "application data directory")?;
    let snapshots = app_data_root.join("context-snapshots");
    ensure_regular_directory(&snapshots, "snapshot storage directory")?;
    let version = snapshots.join("v1");
    ensure_regular_directory(&version, "snapshot schema directory")?;
    Ok(version)
}

fn ensure_store_layout(app_data_root: &Path, workspace_key: &str) -> Result<(), String> {
    let version_root = ensure_snapshot_version_root(app_data_root)?;
    let workspaces = version_root.join("workspaces");
    ensure_regular_directory(&workspaces, "snapshot workspaces directory")?;
    let root = workspaces.join(workspace_key);
    ensure_regular_directory(&root, "workspace snapshot storage")?;
    ensure_regular_directory(&root.join("objects"), "snapshot objects directory")?;
    Ok(())
}

fn validate_existing_store_layout(root: &Path) -> Result<(), String> {
    if !path_entry_exists(root)? {
        return Ok(());
    }
    validate_regular_directory(root, "workspace snapshot storage")?;
    let objects = root.join("objects");
    if path_entry_exists(&objects)? {
        validate_regular_directory(&objects, "snapshot objects directory")?;
    }
    Ok(())
}

fn ensure_regular_directory(path: &Path, label: &str) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(_) => validate_regular_directory(path, label),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(path)
                .map_err(|error| format!("Unable to create {label}: {error}"))?;
            validate_regular_directory(path, label)
        }
        Err(error) => Err(format!("Unable to inspect {label}: {error}")),
    }
}

fn validate_regular_directory(path: &Path, label: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Unable to inspect {label}: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!(
            "The {label} is not a regular app-managed directory."
        ));
    }
    Ok(())
}

fn validate_optional_regular_file(path: &Path, label: &str) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(_) => validate_regular_file(path, label),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Unable to inspect {label}: {error}")),
    }
}

fn validate_regular_file(path: &Path, label: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Unable to inspect {label}: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("The {label} is not a regular file."));
    }
    Ok(())
}

fn path_entry_exists(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("Unable to inspect snapshot storage: {error}")),
    }
}

fn object_path(root: &Path, snapshot_id: &str) -> Result<PathBuf, String> {
    validate_hex_identifier(snapshot_id, 64, "snapshot")?;
    Ok(root.join("objects").join(format!("{snapshot_id}.json")))
}

fn read_json<T: DeserializeOwned>(path: &Path, max_bytes: u64, label: &str) -> Result<T, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Unable to inspect {label}: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("The {label} is not a regular file."));
    }
    if metadata.len() > max_bytes {
        return Err(format!("The {label} exceeds its size limit."));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)
        .map_err(|error| format!("Unable to open {label}: {error}"))?
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Unable to read {label}: {error}"))?;
    if bytes.len() as u64 > max_bytes {
        return Err(format!("The {label} changed and exceeded its size limit."));
    }
    serde_json::from_slice(&bytes).map_err(|error| format!("Unable to parse {label}: {error}"))
}

fn atomic_write_json<T: Serialize>(
    path: &Path,
    value: &T,
    max_bytes: u64,
) -> Result<AtomicWriteOutcome, String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("Unable to serialize snapshot data: {error}"))?;
    if bytes.len() as u64 > max_bytes {
        return Err("The normalized snapshot exceeds its storage limit.".to_string());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "Snapshot storage has no parent directory.".to_string())?;
    validate_regular_directory(parent, "snapshot transaction directory")?;
    let mut temporary = NamedTempFile::new_in(parent)
        .map_err(|error| format!("Unable to create a snapshot transaction: {error}"))?;
    temporary
        .as_file_mut()
        .write_all(&bytes)
        .and_then(|_| temporary.as_file_mut().sync_all())
        .map_err(|error| format!("Unable to write snapshot data: {error}"))?;
    temporary
        .persist(path)
        .map_err(|error| format!("Unable to commit snapshot data: {}", error.error))?;
    let durability_warning = File::open(parent)
        .and_then(|directory| directory.sync_all())
        .err()
        .map(|error| {
            format!("data was committed, but the containing directory could not be synced: {error}")
        });
    Ok(AtomicWriteOutcome { durability_warning })
}

fn garbage_collect_objects(root: &Path, index: &WorkspaceIndex) -> Result<(), String> {
    let objects = root.join("objects");
    if !path_entry_exists(&objects)? {
        return Ok(());
    }
    validate_regular_directory(&objects, "snapshot objects directory")?;
    let retained = index
        .captures
        .iter()
        .map(|record| format!("{}.json", record.snapshot_id))
        .collect::<HashSet<_>>();
    for entry in fs::read_dir(&objects)
        .map_err(|error| format!("Unable to inspect snapshot objects: {error}"))?
    {
        let entry = entry.map_err(|error| format!("Unable to inspect snapshot object: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Unable to inspect snapshot object: {error}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_file() && !retained.contains(&name) {
            fs::remove_file(entry.path())
                .map_err(|error| format!("Unable to remove an expired snapshot object: {error}"))?;
        }
    }
    Ok(())
}

fn remove_uncommitted_object(path: &Path) -> Result<(), String> {
    validate_regular_file(path, "uncommitted snapshot object")?;
    fs::remove_file(path)
        .map_err(|error| format!("Unable to remove the uncommitted snapshot object: {error}"))?;
    let parent = path
        .parent()
        .ok_or_else(|| "The uncommitted snapshot object has no parent directory.".to_string())?;
    if let Err(error) = File::open(parent).and_then(|directory| directory.sync_all()) {
        return Err(format!(
            "The object was removed, but its directory could not be synced: {error}"
        ));
    }
    Ok(())
}

fn validate_hex_identifier(value: &str, length: usize, label: &str) -> Result<(), String> {
    if value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Ok(());
    }
    Err(format!("The {label} identifier is invalid."))
}

fn bounded_text(value: &str, max_chars: usize, label: &str) -> Result<String, String> {
    if value.chars().count() > max_chars {
        return Err(format!("The {label} exceeds its schema limit."));
    }
    if value.chars().any(|character| {
        character.is_control()
            || matches!(
                character,
                '\u{061c}'
                    | '\u{200e}'
                    | '\u{200f}'
                    | '\u{202a}'..='\u{202e}'
                    | '\u{2066}'..='\u{2069}'
            )
    }) {
        return Err(format!(
            "The {label} contains unsupported control characters."
        ));
    }
    Ok(value.to_string())
}

fn redacted_bounded_text(value: &str, max_chars: usize, label: &str) -> Result<String, String> {
    bounded_text(&crate::redaction::redact(value), max_chars, label)
}

fn persistent_artifact_name(artifact: &HarnessArtifact) -> Result<String, String> {
    let redacted = crate::redaction::redact(&artifact.name);
    let name = if contains_absolute_path_shape(&redacted) {
        format!("{} item", kind_label(&artifact.kind))
    } else {
        redacted
    };
    bounded_text(&name, 240, "artifact name")
}

fn contains_absolute_path_shape(value: &str) -> bool {
    let trimmed = value.trim();
    let bytes = trimmed.as_bytes();
    trimmed.starts_with('/')
        || trimmed.starts_with("~/")
        || trimmed.starts_with("~\\")
        || trimmed.starts_with("\\\\")
        || trimmed.contains("/Users/")
        || trimmed.contains("/home/")
        || trimmed.contains("\\Users\\")
        || bytes.windows(3).any(|window| {
            window[0].is_ascii_alphabetic()
                && window[1] == b':'
                && matches!(window[2], b'/' | b'\\')
        })
}

fn kind_label(kind: &HarnessKind) -> &'static str {
    match kind {
        HarnessKind::Instructions => "instructions",
        HarnessKind::Skill => "skill",
        HarnessKind::Hook => "hook",
        HarnessKind::Agent => "agent",
        HarnessKind::Config => "config",
        HarnessKind::Memory => "memory",
        HarnessKind::Rule => "rule",
        HarnessKind::Workflow => "workflow",
        HarnessKind::Plugin => "plugin",
    }
}

fn provider_label(provider: &HarnessProvider) -> &'static str {
    match provider {
        HarnessProvider::Codex => "codex",
        HarnessProvider::Claude => "claude",
        HarnessProvider::Shared => "shared",
        HarnessProvider::Plugin => "plugin",
    }
}

fn scope_label(scope: &HarnessScope) -> &'static str {
    match scope {
        HarnessScope::User => "user",
        HarnessScope::Repo => "repo",
        HarnessScope::Nested => "nested",
        HarnessScope::Worktree => "worktree",
    }
}

fn severity_order(severity: &WarningSeverity) -> u8 {
    match severity {
        WarningSeverity::Info => 0,
        WarningSeverity::Warning => 1,
        WarningSeverity::Error => 2,
    }
}

fn item_metadata_changed(before: &ContextSnapshotItem, after: &ContextSnapshotItem) -> bool {
    before.name != after.name
        || before.kind != after.kind
        || before.provider != after.provider
        || before.scope != after.scope
        || before.source_label != after.source_label
        || before.size_bytes != after.size_bytes
        || before.duplicate_group_id != after.duplicate_group_id
        || before.counterpart_id != after.counterpart_id
}

fn change(
    kind: SnapshotChangeKind,
    before: Option<ContextSnapshotItem>,
    after: Option<ContextSnapshotItem>,
) -> SnapshotArtifactChange {
    let artifact_id = after
        .as_ref()
        .or(before.as_ref())
        .map(|item| item.id.clone())
        .unwrap_or_default();
    SnapshotArtifactChange {
        artifact_id,
        kind,
        before,
        after,
        content_changed: false,
        resolution_changed: false,
        metadata_changed: false,
    }
}

fn change_order(kind: &SnapshotChangeKind) -> u8 {
    match kind {
        SnapshotChangeKind::Added => 0,
        SnapshotChangeKind::Removed => 1,
        SnapshotChangeKind::Changed => 2,
    }
}

fn change_name(change: &SnapshotArtifactChange) -> &str {
    change
        .after
        .as_ref()
        .or(change.before.as_ref())
        .map(|item| item.name.as_str())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    use super::*;
    use crate::model::{HarnessWarning, WarningSeverity};

    #[test]
    fn repeated_capture_reuses_content_identity_without_reusing_capture_identity() {
        let storage = tempfile::tempdir().expect("storage");
        let workspace = tempfile::tempdir().expect("workspace");
        let snapshot = fixture_snapshot(workspace.path(), "a");

        let first = capture(storage.path(), workspace.path(), &snapshot).expect("capture one");
        let second = capture(storage.path(), workspace.path(), &snapshot).expect("capture two");

        assert_eq!(first.captured.snapshot_id, second.captured.snapshot_id);
        assert_ne!(first.captured.capture_id, second.captured.capture_id);
        assert_eq!(second.history.len(), 2);
    }

    #[test]
    fn content_identity_ignores_capture_branch_and_display_metadata() {
        let storage = tempfile::tempdir().expect("storage");
        let workspace = tempfile::tempdir().expect("workspace");
        let first_snapshot = fixture_snapshot(workspace.path(), "a");
        let mut second_snapshot = first_snapshot.clone();
        second_snapshot.workspace_name = "renamed display".to_string();
        second_snapshot.git_branch = Some("feature/snapshot-history".to_string());

        let first =
            capture(storage.path(), workspace.path(), &first_snapshot).expect("first capture");
        let second =
            capture(storage.path(), workspace.path(), &second_snapshot).expect("second capture");

        assert_eq!(first.captured.snapshot_id, second.captured.snapshot_id);
        assert_ne!(
            first.captured.workspace_name,
            second.captured.workspace_name
        );
        assert_ne!(first.captured.git_branch, second.captured.git_branch);
    }

    #[test]
    fn captures_real_scanner_artifact_identifiers() {
        let storage = tempfile::tempdir().expect("storage");
        let workspace = tempfile::tempdir().expect("workspace");
        let home = tempfile::tempdir().expect("home");
        fs::create_dir_all(workspace.path().join(".codex")).expect("codex directory");
        fs::write(workspace.path().join("AGENTS.md"), "# Project rules\n")
            .expect("instructions fixture");
        fs::write(
            workspace.path().join(".codex/config.toml"),
            "model = \"test\"\n",
        )
        .expect("config fixture");
        let snapshot = crate::scanner::scan(workspace.path(), home.path()).expect("real scan");
        assert!(snapshot
            .artifacts
            .iter()
            .all(|artifact| artifact.id.len() == ARTIFACT_ID_LENGTH));

        let captured =
            capture(storage.path(), workspace.path(), &snapshot).expect("real scanner capture");
        let stored = load(
            storage.path(),
            workspace.path(),
            &captured.captured.capture_id,
        )
        .expect("load real scanner capture");

        assert_eq!(stored.items.len(), snapshot.artifacts.len());
    }

    #[test]
    fn durable_projection_excludes_paths_and_content() {
        let storage = tempfile::tempdir().expect("storage");
        let workspace = tempfile::tempdir().expect("workspace");
        let snapshot = fixture_snapshot(workspace.path(), "super-secret-body");

        capture(storage.path(), workspace.path(), &snapshot).expect("capture");
        let key = workspace_key(workspace.path()).expect("workspace key");
        let root = workspace_store_root(storage.path(), &key);
        let mut serialized = String::new();
        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
        {
            if entry.file_type().is_file() {
                serialized.push_str(&fs::read_to_string(entry.path()).expect("stored json"));
            }
        }

        assert!(!serialized.contains(&workspace.path().to_string_lossy().to_string()));
        assert!(!serialized.contains("super-secret-body"));
        assert!(!serialized.contains("workspacePath"));
        assert!(!serialized.contains("content\""));
    }

    #[test]
    fn durable_projection_redacts_secrets_and_generalizes_absolute_path_names() {
        let storage = tempfile::tempdir().expect("storage");
        let workspace = tempfile::tempdir().expect("workspace");
        let token = "github_pat_0123456789abcdefghijklmnopqrstuv";
        let mut secret_snapshot = fixture_snapshot(workspace.path(), "safe");
        secret_snapshot.artifacts[0].name = format!("deploy {token}");
        let secret_capture = capture(storage.path(), workspace.path(), &secret_snapshot)
            .expect("secret name capture")
            .captured;
        let secret_stored = load(storage.path(), workspace.path(), &secret_capture.capture_id)
            .expect("secret name load");
        assert!(!secret_stored.items[0].name.contains(token));
        assert!(secret_stored.items[0].name.contains("<redacted>"));

        let mut path_snapshot = fixture_snapshot(workspace.path(), "safe");
        path_snapshot.artifacts[0].name = "/Users/alice/private/AGENTS.md".to_string();
        let path_capture = capture(storage.path(), workspace.path(), &path_snapshot)
            .expect("path name capture")
            .captured;
        let path_stored = load(storage.path(), workspace.path(), &path_capture.capture_id)
            .expect("path name load");
        assert_eq!(path_stored.items[0].name, "instructions item");

        let key = workspace_key(workspace.path()).expect("workspace key");
        let root = workspace_store_root(storage.path(), &key);
        let serialized = walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| fs::read_to_string(entry.path()).expect("stored json"))
            .collect::<String>();
        assert!(!serialized.contains(token));
        assert!(!serialized.contains("/Users/alice/private"));
    }

    #[test]
    fn history_is_bounded_and_workspace_isolated() {
        let storage = tempfile::tempdir().expect("storage");
        let workspace = tempfile::tempdir().expect("workspace");
        let other = tempfile::tempdir().expect("other workspace");
        let snapshot = fixture_snapshot(workspace.path(), "a");
        for _ in 0..55 {
            capture(storage.path(), workspace.path(), &snapshot).expect("capture");
        }
        let history = list(storage.path(), workspace.path()).expect("history");
        assert_eq!(history.len(), MAX_CAPTURES_PER_WORKSPACE);
        assert!(list(storage.path(), other.path())
            .expect("other history")
            .is_empty());
        assert!(load(storage.path(), other.path(), &history[0].capture_id).is_err());
    }

    #[test]
    fn comparison_reports_content_resolution_metadata_and_diagnostics() {
        let storage = tempfile::tempdir().expect("storage");
        let workspace = tempfile::tempdir().expect("workspace");
        let base_snapshot = fixture_snapshot(workspace.path(), "a");
        let base = capture(storage.path(), workspace.path(), &base_snapshot)
            .expect("base capture")
            .captured;
        let mut target_snapshot = fixture_snapshot(workspace.path(), "b");
        target_snapshot.artifacts[0].resolution = ResolutionState::Shadowed;
        target_snapshot.artifacts[0].size_bytes += 1;
        target_snapshot.warnings.push(HarnessWarning {
            id: "new-diagnostic".to_string(),
            severity: WarningSeverity::Info,
            title: "not persisted".to_string(),
            detail: "not persisted".to_string(),
            artifact_ids: vec![target_snapshot.artifacts[0].id.clone()],
        });
        let target = capture(storage.path(), workspace.path(), &target_snapshot)
            .expect("target capture")
            .captured;

        let comparison = compare(
            storage.path(),
            workspace.path(),
            &base.capture_id,
            &target.capture_id,
        )
        .expect("comparison");
        assert_eq!(comparison.changes.len(), 1);
        assert!(comparison.changes[0].content_changed);
        assert!(comparison.changes[0].resolution_changed);
        assert!(comparison.changes[0].metadata_changed);
        assert!(comparison.diagnostics_changed);
    }

    #[test]
    fn tampered_object_fails_closed() {
        let storage = tempfile::tempdir().expect("storage");
        let workspace = tempfile::tempdir().expect("workspace");
        let snapshot = fixture_snapshot(workspace.path(), "a");
        let captured = capture(storage.path(), workspace.path(), &snapshot)
            .expect("capture")
            .captured;
        let key = workspace_key(workspace.path()).expect("workspace key");
        let root = workspace_store_root(storage.path(), &key);
        let path = object_path(&root, &captured.snapshot_id).expect("object path");
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("object")).expect("json");
        value["payload"]["complete"] = serde_json::Value::Bool(false);
        fs::write(&path, serde_json::to_vec(&value).expect("json")).expect("tamper");

        assert!(load(storage.path(), workspace.path(), &captured.capture_id)
            .expect_err("tampering must fail")
            .contains("integrity"));
    }

    #[test]
    fn corrupted_history_prevents_a_new_capture_from_being_committed() {
        let storage = tempfile::tempdir().expect("storage");
        let workspace = tempfile::tempdir().expect("workspace");
        let first_snapshot = fixture_snapshot(workspace.path(), "a");
        let first = capture(storage.path(), workspace.path(), &first_snapshot)
            .expect("capture")
            .captured;
        let key = workspace_key(workspace.path()).expect("workspace key");
        let root = workspace_store_root(storage.path(), &key);
        let path = object_path(&root, &first.snapshot_id).expect("object path");
        fs::write(path, b"{broken").expect("corrupt object");
        let object_count_before = regular_object_count(&root);

        let second_snapshot = fixture_snapshot(workspace.path(), "different");
        assert!(capture(storage.path(), workspace.path(), &second_snapshot).is_err());
        let index: WorkspaceIndex =
            read_json(&root.join("index.json"), MAX_INDEX_BYTES, "index").expect("index");
        assert_eq!(index.captures.len(), 1);
        assert_eq!(index.captures[0].capture_id, first.capture_id);
        assert_eq!(regular_object_count(&root), object_count_before);
    }

    #[test]
    fn invalid_capture_metadata_does_not_create_an_object_or_change_the_index() {
        let storage = tempfile::tempdir().expect("storage");
        let workspace = tempfile::tempdir().expect("workspace");
        capture(
            storage.path(),
            workspace.path(),
            &fixture_snapshot(workspace.path(), "initial"),
        )
        .expect("initial capture");
        let key = workspace_key(workspace.path()).expect("workspace key");
        let root = workspace_store_root(storage.path(), &key);
        let index_path = root.join("index.json");
        let index_before = fs::read(&index_path).expect("initial index");
        let object_count_before = regular_object_count(&root);

        let mut oversized_name = fixture_snapshot(workspace.path(), "different-name-content");
        oversized_name.workspace_name = "w".repeat(161);
        assert!(capture(storage.path(), workspace.path(), &oversized_name)
            .expect_err("oversized workspace name must fail")
            .contains("workspace name exceeds"));

        let mut invalid_branch = fixture_snapshot(workspace.path(), "different-branch-content");
        invalid_branch.git_branch = Some("feature/unsafe\u{0000}branch".to_string());
        assert!(capture(storage.path(), workspace.path(), &invalid_branch)
            .expect_err("control character in branch must fail")
            .contains("Git branch contains"));

        assert_eq!(regular_object_count(&root), object_count_before);
        assert_eq!(fs::read(index_path).expect("unchanged index"), index_before);
    }

    #[cfg(unix)]
    #[test]
    fn workspace_store_root_symlink_fails_closed() {
        let storage = tempfile::tempdir().expect("storage");
        let workspace = tempfile::tempdir().expect("workspace");
        let external = tempfile::tempdir().expect("external");
        let key = workspace_key(workspace.path()).expect("workspace key");
        let root = workspace_store_root(storage.path(), &key);
        fs::create_dir_all(root.parent().expect("workspaces directory"))
            .expect("workspaces directory");
        symlink(external.path(), &root).expect("root symlink");

        let error = list(storage.path(), workspace.path()).expect_err("symlink must fail");
        assert!(error.contains("not a regular app-managed directory"));
    }

    #[cfg(unix)]
    #[test]
    fn objects_symlink_fails_closed_without_touching_external_files() {
        let storage = tempfile::tempdir().expect("storage");
        let workspace = tempfile::tempdir().expect("workspace");
        let external = tempfile::tempdir().expect("external");
        let sentinel = external.path().join("stale.json");
        fs::write(&sentinel, b"keep").expect("external sentinel");
        let key = workspace_key(workspace.path()).expect("workspace key");
        let root = workspace_store_root(storage.path(), &key);
        fs::create_dir_all(&root).expect("workspace root");
        symlink(external.path(), root.join("objects")).expect("objects symlink");

        let error = capture(
            storage.path(),
            workspace.path(),
            &fixture_snapshot(workspace.path(), "a"),
        )
        .expect_err("objects symlink must fail");
        assert!(error.contains("snapshot objects directory"));
        assert_eq!(fs::read(&sentinel).expect("external sentinel"), b"keep");
    }

    #[cfg(unix)]
    #[test]
    fn file_lock_excludes_a_second_store_process_handle() {
        let storage = tempfile::tempdir().expect("storage");
        let workspace = tempfile::tempdir().expect("workspace");
        let key = workspace_key(workspace.path()).expect("workspace key");
        let held_lock = acquire_store_lock(storage.path(), &key).expect("first lock");
        let lock_path = storage
            .path()
            .join("context-snapshots")
            .join("v1")
            .join("locks")
            .join(format!("{key}.lock"));
        let second_handle = OpenOptions::new()
            .read(true)
            .write(true)
            .open(lock_path)
            .expect("second lock handle");
        assert!(second_handle.try_lock_exclusive().is_err());
        drop(held_lock);
        second_handle
            .try_lock_exclusive()
            .expect("lock becomes available");
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_failure_is_reported_after_commit_and_retried_later() {
        use std::os::unix::fs::PermissionsExt;

        let storage = tempfile::tempdir().expect("storage");
        let workspace = tempfile::tempdir().expect("workspace");
        let snapshot = fixture_snapshot(workspace.path(), "a");
        capture(storage.path(), workspace.path(), &snapshot).expect("initial capture");
        let key = workspace_key(workspace.path()).expect("workspace key");
        let root = workspace_store_root(storage.path(), &key);
        let objects = root.join("objects");
        let stale = objects.join(format!("{}.json", "f".repeat(64)));
        fs::write(&stale, b"stale").expect("stale object");
        fs::set_permissions(&objects, fs::Permissions::from_mode(0o500))
            .expect("read-only objects");

        let outcome = capture(storage.path(), workspace.path(), &snapshot)
            .expect("capture commits despite cleanup failure");
        assert!(outcome.storage_status.cleanup_pending);
        assert!(outcome.storage_status.cleanup_warning.is_some());
        assert!(stale.exists());

        fs::set_permissions(&objects, fs::Permissions::from_mode(0o700)).expect("writable objects");
        list(storage.path(), workspace.path()).expect("later access retries cleanup");
        assert!(!stale.exists());
    }

    #[cfg(unix)]
    #[test]
    fn failed_index_commit_removes_the_new_uncommitted_object() {
        use std::os::unix::fs::PermissionsExt;

        let storage = tempfile::tempdir().expect("storage");
        let workspace = tempfile::tempdir().expect("workspace");
        capture(
            storage.path(),
            workspace.path(),
            &fixture_snapshot(workspace.path(), "a"),
        )
        .expect("initial capture");
        let key = workspace_key(workspace.path()).expect("workspace key");
        let root = workspace_store_root(storage.path(), &key);
        let object_count_before = regular_object_count(&root);
        fs::set_permissions(&root, fs::Permissions::from_mode(0o500))
            .expect("read-only workspace store");

        let result = capture(
            storage.path(),
            workspace.path(),
            &fixture_snapshot(workspace.path(), "different"),
        );
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
            .expect("restore workspace store");

        assert!(result.is_err());
        assert_eq!(regular_object_count(&root), object_count_before);
        assert_eq!(
            list(storage.path(), workspace.path())
                .expect("original history")
                .len(),
            1
        );
    }

    fn regular_object_count(root: &Path) -> usize {
        fs::read_dir(root.join("objects"))
            .expect("objects")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
            .count()
    }

    fn fixture_snapshot(workspace: &Path, content: &str) -> HarnessSnapshot {
        let workspace = workspace.canonicalize().expect("canonical workspace");
        HarnessSnapshot {
            workspace_path: workspace.to_string_lossy().into_owned(),
            workspace_name: "fixture".to_string(),
            git_branch: Some("main".to_string()),
            scanned_at: "2026-08-13T00:00:00Z".to_string(),
            artifacts: vec![HarnessArtifact {
                id: hex::encode(Sha256::digest(b"artifact"))[..ARTIFACT_ID_LENGTH].to_string(),
                name: "AGENTS.md".to_string(),
                kind: HarnessKind::Instructions,
                provider: HarnessProvider::Codex,
                scope: HarnessScope::Repo,
                path: workspace.join("AGENTS.md").to_string_lossy().into_owned(),
                relative_path: "./AGENTS.md".to_string(),
                content: Some(content.to_string()),
                content_hash: hex::encode(Sha256::digest(content.as_bytes())),
                modified_at: None,
                size_bytes: content.len() as u64,
                resolution: ResolutionState::Effective,
                resolution_reason: "not persisted".to_string(),
                duplicate_group_id: None,
                counterpart_id: None,
                description: Some("not persisted".to_string()),
                sensitive: false,
                truncated: false,
                editable: false,
                editability_reason: None,
            }],
            warnings: Vec::new(),
        }
    }
}
