use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HarnessKind {
    Instructions,
    Skill,
    Hook,
    Agent,
    Config,
    Memory,
    Rule,
    Workflow,
    Plugin,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HarnessProvider {
    Codex,
    Claude,
    Shared,
    Plugin,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HarnessScope {
    User,
    Repo,
    Nested,
    Worktree,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ResolutionState {
    Effective,
    Defined,
    Shadowed,
    Duplicate,
    Drifted,
    InstalledInactive,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessArtifact {
    pub id: String,
    pub name: String,
    pub kind: HarnessKind,
    pub provider: HarnessProvider,
    pub scope: HarnessScope,
    pub path: String,
    pub relative_path: String,
    pub content: Option<String>,
    pub content_hash: String,
    pub modified_at: Option<String>,
    pub size_bytes: u64,
    pub resolution: ResolutionState,
    pub resolution_reason: String,
    pub duplicate_group_id: Option<String>,
    pub counterpart_id: Option<String>,
    pub description: Option<String>,
    pub sensitive: bool,
    pub truncated: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessWarning {
    pub id: String,
    pub severity: WarningSeverity,
    pub title: String,
    pub detail: String,
    pub artifact_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WarningSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessSnapshot {
    pub workspace_path: String,
    pub workspace_name: String,
    pub git_branch: Option<String>,
    pub scanned_at: String,
    pub artifacts: Vec<HarnessArtifact>,
    pub warnings: Vec<HarnessWarning>,
}
