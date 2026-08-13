use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    process::{Command, Output},
};

use serde::Serialize;

use crate::model::{
    HarnessKind, HarnessProvider, HarnessSnapshot, ResolutionState, WarningSeverity,
};

const REPORT_SCHEMA_VERSION: u32 = 1;
const PRIVACY_NOTICE: &str = "Aggregate metadata only. Review before sharing; counts can still reveal information about your setup.";
const PROVIDER_LABELS: [&str; 4] = ["codex", "claude", "shared", "plugin"];
const KIND_LABELS: [&str; 9] = [
    "instructions",
    "skill",
    "hook",
    "agent",
    "config",
    "memory",
    "rule",
    "workflow",
    "plugin",
];
const RESOLUTION_LABELS: [&str; 6] = [
    "effective",
    "defined",
    "shadowed",
    "duplicate",
    "installedInactive",
    "unknown",
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReportSource {
    revision: Option<String>,
    dirty: Option<bool>,
}

impl ReportSource {
    /// Detects the Harness Lens source checkout located from this command's build manifest.
    /// The revision is the checkout HEAD observed when the report runs, not build provenance.
    /// All command errors and output other than a validated commit identifier are discarded.
    pub fn detect_from_build_checkout() -> Self {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .filter(|root| is_harness_lens_source_root(root))
            .map_or_else(Self::default, Self::detect_from)
    }

    fn detect_from(root: &Path) -> Self {
        if !is_git_worktree_root(root) {
            return Self::default();
        }
        let revision = git_output(root, &["rev-parse", "--verify", "HEAD^{commit}"])
            .and_then(|output| validated_revision(&output.stdout));
        let dirty = git_output(
            root,
            &[
                "status",
                "--porcelain=v1",
                "--untracked-files=normal",
                "--ignore-submodules=none",
            ],
        )
        .map(|output| !output.stdout.is_empty());

        Self { revision, dirty }
    }

    #[cfg(test)]
    fn new(revision: Option<&str>, dirty: Option<bool>) -> Self {
        Self {
            revision: revision.map(str::to_string),
            dirty,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregateCompatibilityReport {
    pub report_schema_version: u32,
    pub harness_lens_version: String,
    pub source_revision: Option<String>,
    pub source_dirty: Option<bool>,
    pub operating_system: String,
    pub architecture: String,
    pub artifact_count: usize,
    pub by_provider: BTreeMap<String, usize>,
    pub by_kind: BTreeMap<String, usize>,
    pub by_resolution: BTreeMap<String, usize>,
    pub warning_counts: WarningCounts,
    pub scan_complete: bool,
    pub privacy_notice: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarningCounts {
    pub info: usize,
    pub warning: usize,
    pub error: usize,
}

impl AggregateCompatibilityReport {
    pub fn from_snapshot(snapshot: &HarnessSnapshot, source: ReportSource) -> Self {
        let mut by_provider = BTreeMap::new();
        let mut by_kind = BTreeMap::new();
        let mut by_resolution = BTreeMap::new();

        for artifact in &snapshot.artifacts {
            increment(&mut by_provider, provider_label(&artifact.provider));
            increment(&mut by_kind, kind_label(&artifact.kind));
            increment(&mut by_resolution, resolution_label(&artifact.resolution));
        }

        let mut warning_counts = WarningCounts::default();
        let mut scan_complete = true;
        for warning in &snapshot.warnings {
            match warning.severity {
                WarningSeverity::Info => warning_counts.info += 1,
                WarningSeverity::Warning => warning_counts.warning += 1,
                WarningSeverity::Error => {
                    warning_counts.error += 1;
                    scan_complete = false;
                }
            }
            if warning.id == "scan-incomplete" {
                scan_complete = false;
            }
        }

        Self {
            report_schema_version: REPORT_SCHEMA_VERSION,
            harness_lens_version: env!("CARGO_PKG_VERSION").to_string(),
            source_revision: source.revision,
            source_dirty: source.dirty,
            operating_system: std::env::consts::OS.to_string(),
            architecture: public_architecture().to_string(),
            artifact_count: snapshot.artifacts.len(),
            by_provider,
            by_kind,
            by_resolution,
            warning_counts,
            scan_complete,
            privacy_notice: PRIVACY_NOTICE.to_string(),
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut lines = vec![
            "# Harness Lens compatibility report".to_string(),
            String::new(),
            format!("- Report schema: {}", self.report_schema_version),
            format!("- Harness Lens: {}", self.harness_lens_version),
            format!(
                "- Source revision: {}",
                self.source_revision.as_deref().unwrap_or("unknown")
            ),
            format!("- Source dirty: {}", optional_yes_no(self.source_dirty)),
            format!(
                "- Platform: {} / {}",
                self.operating_system, self.architecture
            ),
            format!("- Scan complete: {}", yes_no(self.scan_complete)),
            format!("- Artifacts discovered: {}", self.artifact_count),
            format!(
                "- Warnings: {} info / {} warning / {} error",
                self.warning_counts.info, self.warning_counts.warning, self.warning_counts.error
            ),
        ];

        append_counts(&mut lines, "By provider", &self.by_provider);
        append_counts(&mut lines, "By kind", &self.by_kind);
        append_counts(&mut lines, "By resolution", &self.by_resolution);
        lines.extend([
            "## Evidence boundary".to_string(),
            String::new(),
            "This report describes discovery and static resolution metadata. It does not prove that an Agent used an item or that a task succeeded.".to_string(),
            String::new(),
            format!("_{}_", self.privacy_notice),
        ]);
        lines.join("\n")
    }
}

fn increment(counts: &mut BTreeMap<String, usize>, key: &str) {
    *counts.entry(key.to_string()).or_default() += 1;
}

fn is_harness_lens_source_root(root: &Path) -> bool {
    let Ok(package) = fs::read(root.join("package.json")) else {
        return false;
    };
    let Ok(package) = serde_json::from_slice::<serde_json::Value>(&package) else {
        return false;
    };
    package.get("name").and_then(serde_json::Value::as_str) == Some("harness-lens")
        && root.join("src-tauri/Cargo.toml").is_file()
}

fn git_output(root: &Path, arguments: &[&str]) -> Option<Output> {
    Command::new("git")
        .args(arguments)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .ok()
        .filter(|output| output.status.success())
}

fn is_git_worktree_root(root: &Path) -> bool {
    let Some(output) = git_output(root, &["rev-parse", "--show-toplevel"]) else {
        return false;
    };
    let Ok(reported_root) = std::str::from_utf8(&output.stdout) else {
        return false;
    };
    root.canonicalize().ok() == Path::new(reported_root.trim()).canonicalize().ok()
}

fn validated_revision(output: &[u8]) -> Option<String> {
    let revision = std::str::from_utf8(output).ok()?.trim();
    ((revision.len() == 40 || revision.len() == 64)
        && revision.bytes().all(|byte| byte.is_ascii_hexdigit()))
    .then(|| revision.to_ascii_lowercase())
}

fn provider_label(provider: &HarnessProvider) -> &'static str {
    match provider {
        HarnessProvider::Codex => PROVIDER_LABELS[0],
        HarnessProvider::Claude => PROVIDER_LABELS[1],
        HarnessProvider::Shared => PROVIDER_LABELS[2],
        HarnessProvider::Plugin => PROVIDER_LABELS[3],
    }
}

fn kind_label(kind: &HarnessKind) -> &'static str {
    match kind {
        HarnessKind::Instructions => KIND_LABELS[0],
        HarnessKind::Skill => KIND_LABELS[1],
        HarnessKind::Hook => KIND_LABELS[2],
        HarnessKind::Agent => KIND_LABELS[3],
        HarnessKind::Config => KIND_LABELS[4],
        HarnessKind::Memory => KIND_LABELS[5],
        HarnessKind::Rule => KIND_LABELS[6],
        HarnessKind::Workflow => KIND_LABELS[7],
        HarnessKind::Plugin => KIND_LABELS[8],
    }
}

fn resolution_label(resolution: &ResolutionState) -> &'static str {
    match resolution {
        ResolutionState::Effective => RESOLUTION_LABELS[0],
        ResolutionState::Defined => RESOLUTION_LABELS[1],
        ResolutionState::Shadowed => RESOLUTION_LABELS[2],
        ResolutionState::Duplicate => RESOLUTION_LABELS[3],
        ResolutionState::InstalledInactive => RESOLUTION_LABELS[4],
        ResolutionState::Unknown => RESOLUTION_LABELS[5],
    }
}

fn append_counts(lines: &mut Vec<String>, heading: &str, counts: &BTreeMap<String, usize>) {
    lines.extend([String::new(), format!("## {heading}"), String::new()]);
    if counts.is_empty() {
        lines.push("- None".to_string());
        return;
    }
    lines.extend(
        counts
            .iter()
            .map(|(label, count)| format!("- {label}: {count}")),
    );
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn optional_yes_no(value: Option<bool>) -> &'static str {
    value.map_or("unknown", yes_no)
}

fn public_architecture() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        architecture => architecture,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        HarnessArtifact, HarnessKind, HarnessProvider, HarnessScope, HarnessWarning,
        ResolutionState,
    };

    #[test]
    fn report_excludes_sensitive_snapshot_fields() {
        let snapshot = sensitive_snapshot();
        let report = AggregateCompatibilityReport::from_snapshot(
            &snapshot,
            ReportSource::new(Some("0123456789abcdef0123456789abcdef01234567"), Some(true)),
        );
        let json = serde_json::to_string(&report).expect("serialize report");
        let markdown = report.to_markdown();

        for output in [&json, &markdown] {
            assert!(!output.contains("/Users/private/customer-repository"));
            assert!(!output.contains("customer-repository"));
            assert!(!output.contains("feature/private-client"));
            assert!(!output.contains("1986-05-04T03:02:01Z"));
            assert!(!output.contains("private-artifact-id"));
            assert!(!output.contains("private-relative-source-label.md"));
            assert!(!output.contains("Secret project rule"));
            assert!(!output.contains("super-secret-token"));
            assert!(!output.contains("deadbeef"));
            assert!(!output.contains("1987-06-05T04:03:02Z"));
            assert!(!output.contains("831947"));
            assert!(!output.contains("private-resolution-rationale"));
            assert!(!output.contains("private-duplicate-group"));
            assert!(!output.contains("private-counterpart"));
            assert!(!output.contains("private description"));
            assert!(!output.contains("private-editability-reason"));
            assert!(!output.contains("counterpart-difference:Repo:private"));
            assert!(!output.contains("Same-name content differs"));
        }
        assert_eq!(report.artifact_count, 1);
        assert_eq!(report.by_provider.get("codex"), Some(&1));
        assert_eq!(report.by_kind.get("instructions"), Some(&1));
        assert_eq!(report.by_resolution.get("effective"), Some(&1));
        assert_eq!(
            report.source_revision.as_deref(),
            Some("0123456789abcdef0123456789abcdef01234567")
        );
        assert_eq!(report.source_dirty, Some(true));
    }

    #[test]
    fn incomplete_or_error_diagnostics_keep_scan_incomplete() {
        let mut snapshot = sensitive_snapshot();
        snapshot.warnings = vec![
            HarnessWarning {
                id: "scan-incomplete".to_string(),
                severity: WarningSeverity::Warning,
                title: "private path failed".to_string(),
                detail: "/private/path".to_string(),
                artifact_ids: Vec::new(),
            },
            HarnessWarning {
                id: "runtime-error".to_string(),
                severity: WarningSeverity::Error,
                title: "runtime error".to_string(),
                detail: "secret payload".to_string(),
                artifact_ids: Vec::new(),
            },
        ];

        let report =
            AggregateCompatibilityReport::from_snapshot(&snapshot, ReportSource::default());

        assert!(!report.scan_complete);
        assert_eq!(report.warning_counts.warning, 1);
        assert_eq!(report.warning_counts.error, 1);
        assert!(!report.to_markdown().contains("private path failed"));
        assert!(!report.to_markdown().contains("secret payload"));
    }

    #[test]
    fn json_contract_matches_schema_version_one_allowlist() {
        let report = AggregateCompatibilityReport::from_snapshot(
            &sensitive_snapshot(),
            ReportSource::default(),
        );
        let value = serde_json::to_value(report).expect("serialize report");
        let object = value.as_object().expect("report object");
        let serialized_keys = object
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let schema = serde_json::from_str::<serde_json::Value>(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/schemas/compatibility-report-v1.schema.json"
        )))
        .expect("parse compatibility report schema");
        let schema_properties = object_keys(&schema["properties"]);
        let schema_required = string_values(&schema["required"]);

        assert_eq!(serialized_keys, schema_properties);
        assert_eq!(serialized_keys, schema_required);
        assert_eq!(
            object_keys(&object["warningCounts"]),
            object_keys(&schema["properties"]["warningCounts"]["properties"])
        );
        assert_eq!(
            object_keys(&object["warningCounts"]),
            string_values(&schema["properties"]["warningCounts"]["required"])
        );
        assert_eq!(
            string_values(&schema["$defs"]["providerCounts"]["propertyNames"]["enum"]),
            PROVIDER_LABELS.into_iter().map(str::to_string).collect()
        );
        assert_eq!(
            string_values(&schema["$defs"]["kindCounts"]["propertyNames"]["enum"]),
            KIND_LABELS.into_iter().map(str::to_string).collect()
        );
        assert_eq!(
            string_values(&schema["$defs"]["resolutionCounts"]["propertyNames"]["enum"]),
            RESOLUTION_LABELS.into_iter().map(str::to_string).collect()
        );
    }

    #[test]
    fn unknown_source_stays_explicit_and_revision_validation_is_content_free() {
        let report = AggregateCompatibilityReport::from_snapshot(
            &sensitive_snapshot(),
            ReportSource::default(),
        );
        let value = serde_json::to_value(&report).expect("serialize report");

        assert!(value["sourceRevision"].is_null());
        assert!(value["sourceDirty"].is_null());
        assert!(report.to_markdown().contains("Source revision: unknown"));
        assert!(report.to_markdown().contains("Source dirty: unknown"));
        assert_eq!(
            validated_revision(b"ABCDEF0123456789ABCDEF0123456789ABCDEF01\n").as_deref(),
            Some("abcdef0123456789abcdef0123456789abcdef01")
        );
        assert!(validated_revision(b"/Users/private/customer-repository").is_none());
        assert!(validated_revision(b"fatal: private source detail").is_none());
    }

    fn object_keys(value: &serde_json::Value) -> std::collections::BTreeSet<String> {
        value
            .as_object()
            .expect("schema object")
            .keys()
            .cloned()
            .collect()
    }

    fn string_values(value: &serde_json::Value) -> std::collections::BTreeSet<String> {
        let values = value.as_array().expect("schema string array");
        assert_eq!(
            values.len(),
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            "schema string array must contain unique strings"
        );
        values
            .iter()
            .map(|value| value.as_str().expect("schema string").to_string())
            .collect()
    }

    fn sensitive_snapshot() -> HarnessSnapshot {
        HarnessSnapshot {
            workspace_path: "/Users/private/customer-repository".to_string(),
            workspace_name: "customer-repository".to_string(),
            git_branch: Some("feature/private-client".to_string()),
            scanned_at: "1986-05-04T03:02:01Z".to_string(),
            artifacts: vec![HarnessArtifact {
                id: "private-artifact-id".to_string(),
                name: "Secret project rule".to_string(),
                kind: HarnessKind::Instructions,
                provider: HarnessProvider::Codex,
                scope: HarnessScope::Repo,
                path: "/Users/private/customer-repository/AGENTS.md".to_string(),
                relative_path: "private-relative-source-label.md".to_string(),
                content: Some("token=super-secret-token".to_string()),
                content_hash: "deadbeef".to_string(),
                modified_at: Some("1987-06-05T04:03:02Z".to_string()),
                size_bytes: 831_947,
                resolution: ResolutionState::Effective,
                resolution_reason: "private-resolution-rationale".to_string(),
                duplicate_group_id: Some("private-duplicate-group".to_string()),
                counterpart_id: Some("private-counterpart".to_string()),
                description: Some("private description".to_string()),
                sensitive: true,
                truncated: false,
                editable: false,
                editability_reason: Some("private-editability-reason".to_string()),
            }],
            warnings: vec![HarnessWarning {
                id: "counterpart-difference:Repo:private".to_string(),
                severity: WarningSeverity::Info,
                title: "Same-name content differs: Secret project rule".to_string(),
                detail: "Same-name content differs".to_string(),
                artifact_ids: vec!["private-artifact-id".to_string()],
            }],
        }
    }
}
