use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{BufReader, Read},
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use walkdir::{DirEntry, WalkDir};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use crate::{
    memory_edit::{memory_editability, MAX_MEMORY_BYTES},
    model::{
        HarnessArtifact, HarnessKind, HarnessProvider, HarnessScope, HarnessSnapshot,
        HarnessWarning, ResolutionState, WarningSeverity,
    },
    redaction,
};

const MAX_CONTENT_BYTES: u64 = 256 * 1024;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TOTAL_FILE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_CANDIDATES: usize = 4_096;
const MAX_WALK_ENTRIES_PER_ROOT: usize = 20_000;
const MAX_SCAN_DIAGNOSTICS: usize = 64;
const GUIDANCE_LINE_REVIEW_THRESHOLD: u64 = 200;
const CODEX_PROJECT_INSTRUCTION_BUDGET_BYTES: u64 = 32 * 1024;

#[derive(Clone)]
struct Candidate {
    path: PathBuf,
    name: Option<String>,
    kind: HarnessKind,
    provider: HarnessProvider,
    scope: HarnessScope,
    resolution: ResolutionState,
    reason: String,
    sensitive: bool,
    metadata_only: bool,
}

#[derive(Debug)]
struct ScannedFileContent {
    preview: String,
    truncated: bool,
    content_hash: String,
    line_count: u64,
    size_bytes: u64,
    modified_at: Option<SystemTime>,
    metadata: fs::Metadata,
}

pub fn scan(workspace: &Path, home: &Path) -> Result<HarnessSnapshot, String> {
    let workspace = workspace
        .canonicalize()
        .map_err(|error| format!("Unable to open workspace: {error}"))?;
    if !workspace.is_dir() {
        return Err("The selected workspace is not a directory.".to_string());
    }

    let repo_root = git_root(&workspace).unwrap_or_else(|| workspace.clone());
    let mut candidates = Vec::new();
    let mut scan_diagnostics = Vec::new();
    collect_user_candidates(
        home,
        &workspace,
        &repo_root,
        &mut candidates,
        &mut scan_diagnostics,
    );
    collect_repo_candidates(
        &repo_root,
        &workspace,
        &mut candidates,
        &mut scan_diagnostics,
    );
    deduplicate_candidates(&mut candidates);

    let candidate_count = candidates.len();
    let mut total_file_bytes = 0_u64;
    let mut artifacts = Vec::new();
    for candidate in candidates.into_iter().take(MAX_CANDIDATES) {
        let candidate_path = candidate.path.to_string_lossy().into_owned();
        match materialize(
            candidate,
            &workspace,
            &repo_root,
            home,
            MAX_TOTAL_FILE_BYTES.saturating_sub(total_file_bytes),
        ) {
            Ok(artifact) => {
                total_file_bytes = total_file_bytes.saturating_add(artifact.size_bytes);
                artifacts.push(artifact);
            }
            Err(error) => {
                record_scan_diagnostic(&mut scan_diagnostics, format!("{candidate_path}: {error}"))
            }
        }
    }
    if candidate_count > MAX_CANDIDATES {
        record_scan_diagnostic(
            &mut scan_diagnostics,
            format!("Candidate budget reached ({MAX_CANDIDATES}); additional items were skipped."),
        );
    }
    artifacts.sort_by(|left, right| {
        format!("{:?}-{:?}-{}", left.provider, left.kind, left.name).cmp(&format!(
            "{:?}-{:?}-{}",
            right.provider, right.kind, right.name
        ))
    });

    let mut warnings = annotate_duplicates_and_counterpart_differences(&mut artifacts, &repo_root);
    warnings.extend(quality_warnings(&artifacts));
    if !scan_diagnostics.is_empty() {
        warnings.push(incomplete_scan_warning(&scan_diagnostics));
    }
    warnings.push(HarnessWarning {
        id: "runtime-not-connected".to_string(),
        severity: WarningSeverity::Info,
        title: "Runtime evidence is not connected yet".to_string(),
        detail: "Defined and effective states come from static adapter rules. Actual usage requires a runtime event source.".to_string(),
        artifact_ids: Vec::new(),
    });
    warnings.sort_by(|left, right| {
        warning_severity_order(&left.severity)
            .cmp(&warning_severity_order(&right.severity))
            .then_with(|| left.id.cmp(&right.id))
    });

    Ok(HarnessSnapshot {
        workspace_path: workspace.to_string_lossy().into_owned(),
        workspace_name: workspace
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Workspace")
            .to_string(),
        git_branch: git_branch(&workspace),
        scanned_at: Utc::now().to_rfc3339(),
        artifacts,
        warnings,
    })
}

fn collect_user_candidates(
    home: &Path,
    workspace: &Path,
    repo_root: &Path,
    output: &mut Vec<Candidate>,
    diagnostics: &mut Vec<String>,
) {
    let codex = home.join(".codex");
    let claude = home.join(".claude");

    collect_preferred_instruction(
        &codex,
        HarnessProvider::Codex,
        HarnessScope::User,
        "Global Codex instructions",
        "Loaded as the global Codex instruction source.",
        output,
    );
    push_if_file(
        output,
        codex.join("config.toml"),
        None,
        HarnessKind::Config,
        HarnessProvider::Codex,
        HarnessScope::User,
        ResolutionState::Effective,
        "User Codex configuration participates in the effective config chain.",
        true,
        false,
    );
    push_if_file(
        output,
        codex.join("hooks.json"),
        Some("Codex lifecycle hooks".into()),
        HarnessKind::Hook,
        HarnessProvider::Codex,
        HarnessScope::User,
        ResolutionState::Defined,
        "Discovered; live hook status requires the Codex runtime adapter.",
        true,
        false,
    );
    collect_matching_files(
        &codex.join("rules"),
        2,
        &["rules"],
        HarnessKind::Rule,
        HarnessProvider::Codex,
        HarnessScope::User,
        ResolutionState::Defined,
        "Discovered in the user Codex rules directory.",
        output,
        diagnostics,
    );
    collect_skill_dir(
        &home.join(".agents/skills"),
        HarnessProvider::Shared,
        HarnessScope::User,
        ResolutionState::Effective,
        "Available from the user skill directory.",
        output,
        diagnostics,
    );
    collect_skill_dir(
        &codex.join("skills"),
        HarnessProvider::Codex,
        HarnessScope::User,
        ResolutionState::Defined,
        "Discovered in the Codex-specific skill directory.",
        output,
        diagnostics,
    );
    collect_agent_dir(
        &codex.join("agents"),
        HarnessProvider::Codex,
        HarnessScope::User,
        output,
        diagnostics,
    );
    for name in ["memory_summary.md", "MEMORY.md"] {
        push_if_file(
            output,
            codex.join("memories").join(name),
            None,
            HarnessKind::Memory,
            HarnessProvider::Codex,
            HarnessScope::User,
            ResolutionState::Defined,
            "Memory metadata only; expand deliberately when runtime usage is connected.",
            true,
            true,
        );
    }
    collect_memory_source(
        &codex.join("memories/extensions/ad_hoc"),
        8,
        &["md"],
        HarnessProvider::Codex,
        HarnessScope::User,
        ResolutionState::Defined,
        "User-maintained Codex memory extension discovered; runtime loading is not observed.",
        output,
        diagnostics,
    );

    push_if_file(
        output,
        claude.join("CLAUDE.md"),
        Some("Global CLAUDE.md".into()),
        HarnessKind::Instructions,
        HarnessProvider::Claude,
        HarnessScope::User,
        ResolutionState::Defined,
        "Discovered; actual loading requires Claude runtime evidence.",
        false,
        false,
    );
    for name in ["settings.json", "settings.local.json"] {
        push_if_file(
            output,
            claude.join(name),
            None,
            HarnessKind::Config,
            HarnessProvider::Claude,
            HarnessScope::User,
            ResolutionState::Defined,
            "Discovered in the user Claude configuration directory.",
            true,
            false,
        );
    }
    collect_skill_dir(
        &claude.join("skills"),
        HarnessProvider::Claude,
        HarnessScope::User,
        ResolutionState::Defined,
        "Discovered; actual invocation requires Claude runtime evidence.",
        output,
        diagnostics,
    );
    collect_agent_dir(
        &claude.join("agents"),
        HarnessProvider::Claude,
        HarnessScope::User,
        output,
        diagnostics,
    );
    collect_matching_files(
        &claude.join("rules"),
        8,
        &["md", "txt", "json", "toml", "yaml", "yml"],
        HarnessKind::Rule,
        HarnessProvider::Claude,
        HarnessScope::User,
        ResolutionState::Defined,
        "Discovered in the user Claude rules directory; runtime resolution is not observed.",
        output,
        diagnostics,
    );
    collect_matching_files(
        &claude.join("commands"),
        8,
        &["md"],
        HarnessKind::Skill,
        HarnessProvider::Claude,
        HarnessScope::User,
        ResolutionState::Defined,
        "Legacy Claude command discovered; invocation is not observed.",
        output,
        diagnostics,
    );
    let project_chain = directory_chain(repo_root, workspace);
    collect_claude_project_memories(
        &claude.join("projects"),
        project_chain.iter().map(PathBuf::as_path),
        output,
        diagnostics,
    );
}

fn collect_repo_candidates(
    repo_root: &Path,
    workspace: &Path,
    output: &mut Vec<Candidate>,
    diagnostics: &mut Vec<String>,
) {
    let chain = directory_chain(repo_root, workspace);
    for (index, directory) in chain.iter().enumerate() {
        let scope = if index == 0 {
            HarnessScope::Repo
        } else {
            HarnessScope::Nested
        };
        collect_preferred_instruction(
            directory,
            HarnessProvider::Codex,
            scope.clone(),
            "Project Codex instructions",
            "Included in the Codex instruction chain for this working directory.",
            output,
        );
        push_if_file(
            output,
            directory.join("CLAUDE.md"),
            None,
            HarnessKind::Instructions,
            HarnessProvider::Claude,
            scope.clone(),
            ResolutionState::Defined,
            "Discovered; actual loading requires Claude runtime evidence.",
            false,
            false,
        );
        collect_skill_dir(
            &directory.join(".agents/skills"),
            HarnessProvider::Shared,
            scope.clone(),
            ResolutionState::Effective,
            "Available from the active repository ancestor chain.",
            output,
            diagnostics,
        );
        collect_workflows(
            &directory.join(".agents/skills"),
            scope.clone(),
            output,
            diagnostics,
        );
        collect_project_directory_candidates(directory, scope, output, diagnostics);
    }
}

fn collect_project_directory_candidates(
    directory: &Path,
    scope: HarnessScope,
    output: &mut Vec<Candidate>,
    diagnostics: &mut Vec<String>,
) {
    let codex = directory.join(".codex");
    push_if_file(
        output,
        codex.join("config.toml"),
        None,
        HarnessKind::Config,
        HarnessProvider::Codex,
        scope.clone(),
        ResolutionState::Unknown,
        "Project config is effective only when the runtime trusts this project.",
        true,
        false,
    );
    push_if_file(
        output,
        codex.join("hooks.json"),
        Some("Project Codex hooks".into()),
        HarnessKind::Hook,
        HarnessProvider::Codex,
        scope.clone(),
        ResolutionState::Unknown,
        "Project hooks require trusted-project and runtime status evidence.",
        true,
        false,
    );
    collect_matching_files(
        &codex.join("hooks"),
        8,
        &[
            "json", "toml", "yaml", "yml", "sh", "bash", "zsh", "py", "js", "ts",
        ],
        HarnessKind::Hook,
        HarnessProvider::Codex,
        scope.clone(),
        ResolutionState::Unknown,
        "Project hook script requires trusted-project and runtime status evidence.",
        output,
        diagnostics,
    );
    collect_matching_files(
        &codex.join("rules"),
        8,
        &["rules", "md", "toml", "yaml", "yml", "json"],
        HarnessKind::Rule,
        HarnessProvider::Codex,
        scope.clone(),
        ResolutionState::Defined,
        "Discovered in a project Codex rules directory; runtime resolution is not observed.",
        output,
        diagnostics,
    );
    collect_skill_dir(
        &codex.join("skills"),
        HarnessProvider::Codex,
        scope.clone(),
        ResolutionState::Defined,
        "Discovered in a project Codex skill directory; invocation is not observed.",
        output,
        diagnostics,
    );
    collect_memory_source(
        &codex.join("memories"),
        8,
        &["md", "txt", "json", "jsonl", "toml", "yaml", "yml"],
        HarnessProvider::Codex,
        scope.clone(),
        ResolutionState::Defined,
        "Project memory metadata discovered; runtime loading is not observed.",
        output,
        diagnostics,
    );
    collect_agent_dir(
        &codex.join("agents"),
        HarnessProvider::Codex,
        scope.clone(),
        output,
        diagnostics,
    );

    let claude = directory.join(".claude");
    for name in ["settings.json", "settings.local.json"] {
        push_if_file(
            output,
            claude.join(name),
            None,
            HarnessKind::Config,
            HarnessProvider::Claude,
            scope.clone(),
            ResolutionState::Defined,
            "Discovered; effective status requires Claude runtime evidence.",
            true,
            false,
        );
    }
    collect_skill_dir(
        &claude.join("skills"),
        HarnessProvider::Claude,
        scope.clone(),
        ResolutionState::Defined,
        "Discovered; actual invocation requires Claude runtime evidence.",
        output,
        diagnostics,
    );
    collect_agent_dir(
        &claude.join("agents"),
        HarnessProvider::Claude,
        scope.clone(),
        output,
        diagnostics,
    );
    collect_matching_files(
        &claude.join("rules"),
        8,
        &["md", "txt", "json", "toml", "yaml", "yml"],
        HarnessKind::Rule,
        HarnessProvider::Claude,
        scope.clone(),
        ResolutionState::Defined,
        "Project Claude rule discovered; runtime resolution is not observed.",
        output,
        diagnostics,
    );
    collect_matching_files(
        &claude.join("commands"),
        8,
        &["md"],
        HarnessKind::Skill,
        HarnessProvider::Claude,
        scope.clone(),
        ResolutionState::Defined,
        "Legacy project Claude command discovered; invocation is not observed.",
        output,
        diagnostics,
    );
    for memory_path in [claude.join("memory"), claude.join("memories")] {
        collect_memory_source(
            &memory_path,
            8,
            &["md", "txt", "json", "jsonl", "toml", "yaml", "yml"],
            HarnessProvider::Claude,
            scope.clone(),
            ResolutionState::Defined,
            "Project Claude memory metadata discovered; runtime loading is not observed.",
            output,
            diagnostics,
        );
    }
}

fn collect_claude_project_memories<'a>(
    projects_root: &Path,
    project_paths: impl IntoIterator<Item = &'a Path>,
    output: &mut Vec<Candidate>,
    diagnostics: &mut Vec<String>,
) {
    let project_keys = project_paths
        .into_iter()
        .map(claude_project_key)
        .collect::<std::collections::HashSet<_>>();
    for project_key in project_keys {
        collect_memory_source(
            &projects_root.join(project_key).join("memory"),
            4,
            &["md"],
            HarnessProvider::Claude,
            HarnessScope::Worktree,
            ResolutionState::Defined,
            "Claude project memory metadata discovered; runtime loading is not observed.",
            output,
            diagnostics,
        );
    }
}

fn claude_project_key(path: &Path) -> String {
    path.to_string_lossy().replace('/', "-")
}

fn collect_preferred_instruction(
    directory: &Path,
    provider: HarnessProvider,
    scope: HarnessScope,
    display_name: &str,
    reason: &str,
    output: &mut Vec<Candidate>,
) {
    let override_path = directory.join("AGENTS.override.md");
    let regular_path = directory.join("AGENTS.md");
    let has_override = is_non_empty_file(&override_path);
    let has_regular = is_non_empty_file(&regular_path);

    if has_override {
        push_if_file(
            output,
            override_path,
            Some(format!("{display_name} override")),
            HarnessKind::Instructions,
            provider.clone(),
            scope.clone(),
            ResolutionState::Effective,
            reason,
            false,
            false,
        );
    }

    if has_regular {
        let (resolution, resolution_reason) = if has_override {
            (
                ResolutionState::Shadowed,
                "Defined in the same directory but shadowed by AGENTS.override.md.",
            )
        } else {
            (ResolutionState::Effective, reason)
        };
        push_if_file(
            output,
            regular_path,
            Some(display_name.to_string()),
            HarnessKind::Instructions,
            provider,
            scope,
            resolution,
            resolution_reason,
            false,
            false,
        );
    }
}

fn collect_skill_dir(
    root: &Path,
    provider: HarnessProvider,
    scope: HarnessScope,
    resolution: ResolutionState,
    reason: &str,
    output: &mut Vec<Candidate>,
    diagnostics: &mut Vec<String>,
) {
    if !root.is_dir() {
        return;
    }
    for (index, entry) in WalkDir::new(root)
        .min_depth(2)
        .max_depth(2)
        .into_iter()
        .filter_entry(not_hidden_or_root)
        .enumerate()
    {
        if index >= MAX_WALK_ENTRIES_PER_ROOT {
            record_scan_diagnostic(
                diagnostics,
                format!(
                    "{}: traversal budget reached ({MAX_WALK_ENTRIES_PER_ROOT} entries).",
                    root.to_string_lossy()
                ),
            );
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                record_scan_diagnostic(diagnostics, format!("{}: {error}", root.to_string_lossy()));
                continue;
            }
        };
        if !(entry.file_type().is_file() || entry.file_type().is_symlink())
            || entry.file_name() != "SKILL.md"
        {
            continue;
        }
        push_if_file(
            output,
            entry.into_path(),
            None,
            HarnessKind::Skill,
            provider.clone(),
            scope.clone(),
            resolution.clone(),
            reason,
            false,
            false,
        );
    }
}

fn collect_agent_dir(
    root: &Path,
    provider: HarnessProvider,
    scope: HarnessScope,
    output: &mut Vec<Candidate>,
    diagnostics: &mut Vec<String>,
) {
    collect_matching_files(
        root,
        1,
        &["md", "toml"],
        HarnessKind::Agent,
        provider,
        scope,
        ResolutionState::Defined,
        "Discovered Agent definition; runtime registration is not observed yet.",
        output,
        diagnostics,
    );
}

fn collect_workflows(
    root: &Path,
    scope: HarnessScope,
    output: &mut Vec<Candidate>,
    diagnostics: &mut Vec<String>,
) {
    if !root.is_dir() {
        return;
    }
    for (index, entry) in WalkDir::new(root)
        .min_depth(2)
        .max_depth(4)
        .into_iter()
        .filter_entry(not_hidden_or_root)
        .enumerate()
    {
        if index >= MAX_WALK_ENTRIES_PER_ROOT {
            record_scan_diagnostic(
                diagnostics,
                format!(
                    "{}: traversal budget reached ({MAX_WALK_ENTRIES_PER_ROOT} entries).",
                    root.to_string_lossy()
                ),
            );
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                record_scan_diagnostic(diagnostics, format!("{}: {error}", root.to_string_lossy()));
                continue;
            }
        };
        if !(entry.file_type().is_file() || entry.file_type().is_symlink()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if name.contains(".workflow.") {
            push_if_file(
                output,
                entry.into_path(),
                None,
                HarnessKind::Workflow,
                HarnessProvider::Shared,
                scope.clone(),
                ResolutionState::Defined,
                "Workflow reference discovered; it is not assumed to be executable.",
                false,
                false,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_memory_source(
    root: &Path,
    max_depth: usize,
    extensions: &[&str],
    provider: HarnessProvider,
    scope: HarnessScope,
    resolution: ResolutionState,
    reason: &str,
    output: &mut Vec<Candidate>,
    diagnostics: &mut Vec<String>,
) {
    if root.is_file() {
        push_if_file(
            output,
            root.to_path_buf(),
            None,
            HarnessKind::Memory,
            provider,
            scope,
            resolution,
            reason,
            true,
            true,
        );
        return;
    }
    if !root.is_dir() {
        return;
    }
    for (index, entry) in WalkDir::new(root)
        .min_depth(1)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(not_hidden_or_root)
        .enumerate()
    {
        if index >= MAX_WALK_ENTRIES_PER_ROOT {
            record_scan_diagnostic(
                diagnostics,
                format!(
                    "{}: traversal budget reached ({MAX_WALK_ENTRIES_PER_ROOT} entries).",
                    root.to_string_lossy()
                ),
            );
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                record_scan_diagnostic(diagnostics, format!("{}: {error}", root.to_string_lossy()));
                continue;
            }
        };
        if !(entry.file_type().is_file() || entry.file_type().is_symlink()) {
            continue;
        }
        let extension = entry.path().extension().and_then(|value| value.to_str());
        if extension.is_some_and(|value| extensions.contains(&value)) {
            push_if_file(
                output,
                entry.into_path(),
                None,
                HarnessKind::Memory,
                provider.clone(),
                scope.clone(),
                resolution.clone(),
                reason,
                true,
                true,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_matching_files(
    root: &Path,
    max_depth: usize,
    extensions: &[&str],
    kind: HarnessKind,
    provider: HarnessProvider,
    scope: HarnessScope,
    resolution: ResolutionState,
    reason: &str,
    output: &mut Vec<Candidate>,
    diagnostics: &mut Vec<String>,
) {
    if !root.is_dir() {
        return;
    }
    for (index, entry) in WalkDir::new(root)
        .min_depth(1)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(not_hidden_or_root)
        .enumerate()
    {
        if index >= MAX_WALK_ENTRIES_PER_ROOT {
            record_scan_diagnostic(
                diagnostics,
                format!(
                    "{}: traversal budget reached ({MAX_WALK_ENTRIES_PER_ROOT} entries).",
                    root.to_string_lossy()
                ),
            );
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                record_scan_diagnostic(diagnostics, format!("{}: {error}", root.to_string_lossy()));
                continue;
            }
        };
        if !(entry.file_type().is_file() || entry.file_type().is_symlink()) {
            continue;
        }
        let extension = entry.path().extension().and_then(|value| value.to_str());
        if extension.is_some_and(|value| extensions.contains(&value)) {
            push_if_file(
                output,
                entry.into_path(),
                None,
                kind.clone(),
                provider.clone(),
                scope.clone(),
                resolution.clone(),
                reason,
                false,
                false,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_if_file(
    output: &mut Vec<Candidate>,
    path: PathBuf,
    name: Option<String>,
    kind: HarnessKind,
    provider: HarnessProvider,
    scope: HarnessScope,
    resolution: ResolutionState,
    reason: &str,
    sensitive: bool,
    metadata_only: bool,
) {
    // Keep one sentinel candidate so `scan` can report that discovery exceeded its budget,
    // while bounding candidate allocation even when many roots are present.
    if path.is_file() && output.len() <= MAX_CANDIDATES {
        output.push(Candidate {
            path,
            name,
            kind,
            provider,
            scope,
            resolution,
            reason: reason.to_string(),
            sensitive,
            metadata_only,
        });
    }
}

fn materialize(
    candidate: Candidate,
    workspace: &Path,
    repo_root: &Path,
    home: &Path,
    remaining_total_bytes: u64,
) -> Result<HarnessArtifact, String> {
    let id = artifact_id(&candidate);
    let source_path = candidate.path.clone();
    let allowed_source_root = authorized_root(&candidate, repo_root, home);
    let source_uses_symlink = path_uses_symlink_below_root(&source_path, &allowed_source_root);
    let canonical_path = source_path
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let allowed_root = allowed_source_root
        .canonicalize()
        .map_err(|error| format!("Unable to resolve authorized root: {error}"))?;
    if !canonical_path.starts_with(&allowed_root) {
        return Err(format!(
            "Resolved outside authorized root {} and was skipped.",
            allowed_root.to_string_lossy()
        ));
    }

    let scanned = read_preview_and_hash(
        &canonical_path,
        &allowed_root,
        MAX_CONTENT_BYTES,
        MAX_FILE_BYTES,
        remaining_total_bytes,
    )?;
    let ScannedFileContent {
        preview: raw_content,
        truncated,
        content_hash,
        line_count,
        size_bytes,
        modified_at,
        metadata,
    } = scanned;
    let (frontmatter_name, description) = if candidate.kind == HarnessKind::Skill {
        parse_skill_frontmatter(&raw_content)
    } else {
        (None, None)
    };
    let description = description.map(|value| redaction::redact(&value));
    let is_skill_manifest = candidate.kind == HarnessKind::Skill
        && canonical_path
            .file_name()
            .is_some_and(|name| name == "SKILL.md");
    let inferred_name = if is_skill_manifest {
        canonical_path.parent().and_then(Path::file_name)
    } else {
        canonical_path.file_stem()
    }
    .and_then(|name| name.to_str())
    .unwrap_or("Harness item")
    .to_string();
    let name = candidate.name.or(frontmatter_name).unwrap_or(inferred_name);
    let path_string = canonical_path.to_string_lossy().into_owned();
    let relative_path = canonical_path
        .strip_prefix(workspace)
        .map(|path| format!("./{}", path.to_string_lossy()))
        .unwrap_or_else(|_| path_string.clone());
    let (editable, editability_reason) = if candidate.kind == HarnessKind::Memory {
        if size_bytes > MAX_MEMORY_BYTES {
            (
                false,
                Some(format!(
                    "Memory files above {} KiB must be opened externally and cannot be edited in Harness Lens.",
                    MAX_MEMORY_BYTES / 1024
                )),
            )
        } else if source_uses_symlink {
            (
                false,
                Some("Memory paths that use symbolic links are view-only.".to_string()),
            )
        } else {
            memory_editability(&canonical_path)
        }
    } else {
        (false, None)
    };
    ensure_path_matches_open_file(&canonical_path, &allowed_root, &metadata)?;

    Ok(HarnessArtifact {
        id,
        name,
        kind: candidate.kind,
        provider: candidate.provider,
        scope: candidate.scope,
        path: path_string,
        relative_path,
        content: if candidate.metadata_only {
            None
        } else {
            Some(redaction::redact(&raw_content))
        },
        content_hash,
        modified_at: modified_at.map(system_time_to_rfc3339),
        size_bytes,
        line_count,
        resolution: candidate.resolution,
        resolution_reason: candidate.reason,
        duplicate_group_id: None,
        counterpart_id: None,
        description,
        sensitive: candidate.sensitive,
        truncated,
        editable,
        editability_reason,
    })
}

fn deduplicate_candidates(candidates: &mut Vec<Candidate>) {
    let mut seen = HashSet::new();
    candidates.retain(|candidate| {
        seen.insert((
            provider_id_tag(&candidate.provider),
            kind_id_tag(&candidate.kind),
            candidate.path.clone(),
        ))
    });
}

fn artifact_id(candidate: &Candidate) -> String {
    let mut hasher = Sha256::new();
    hasher.update(provider_id_tag(&candidate.provider).as_bytes());
    hasher.update([0]);
    hasher.update(kind_id_tag(&candidate.kind).as_bytes());
    hasher.update([0]);
    hash_source_path(&mut hasher, &candidate.path);
    hex::encode(hasher.finalize())[..24].to_string()
}

#[cfg(unix)]
fn hash_source_path(hasher: &mut Sha256, path: &Path) {
    use std::os::unix::ffi::OsStrExt;

    hasher.update(path.as_os_str().as_bytes());
}

#[cfg(windows)]
fn hash_source_path(hasher: &mut Sha256, path: &Path) {
    use std::os::windows::ffi::OsStrExt;

    for unit in path.as_os_str().encode_wide() {
        hasher.update(unit.to_le_bytes());
    }
}

#[cfg(not(any(unix, windows)))]
fn hash_source_path(hasher: &mut Sha256, path: &Path) {
    hasher.update(path.to_string_lossy().as_bytes());
}

fn provider_id_tag(provider: &HarnessProvider) -> &'static str {
    match provider {
        HarnessProvider::Codex => "codex",
        HarnessProvider::Claude => "claude",
        HarnessProvider::Shared => "shared",
        HarnessProvider::Plugin => "plugin",
    }
}

fn kind_id_tag(kind: &HarnessKind) -> &'static str {
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

fn authorized_root(candidate: &Candidate, repo_root: &Path, home: &Path) -> PathBuf {
    match candidate.scope {
        HarnessScope::Repo | HarnessScope::Nested => repo_root.to_path_buf(),
        HarnessScope::Worktree => home.join(".claude/projects"),
        HarnessScope::User => match candidate.provider {
            HarnessProvider::Codex => home.join(".codex"),
            HarnessProvider::Claude => home.join(".claude"),
            HarnessProvider::Shared => home.join(".agents"),
            HarnessProvider::Plugin => home.to_path_buf(),
        },
    }
}

fn path_uses_symlink_below_root(path: &Path, root: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return true;
    };
    let mut current = root.to_path_buf();
    if fs::symlink_metadata(&current).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return true;
    }
    for component in relative.components() {
        current.push(component);
        if fs::symlink_metadata(&current).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return true;
        }
    }
    false
}

fn read_preview_and_hash(
    path: &Path,
    allowed_root: &Path,
    preview_limit: u64,
    max_file_bytes: u64,
    remaining_total_bytes: u64,
) -> Result<ScannedFileContent, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let before = file.metadata().map_err(|error| error.to_string())?;
    if !before.is_file() {
        return Err("Resolved path is not a regular file.".to_string());
    }
    if before.len() > max_file_bytes {
        return Err(format!(
            "File exceeds the {} MiB per-file scan limit.",
            max_file_bytes / (1024 * 1024)
        ));
    }
    if before.len() > remaining_total_bytes {
        return Err(format!(
            "Total scan byte budget reached ({} MiB).",
            MAX_TOTAL_FILE_BYTES / (1024 * 1024)
        ));
    }
    ensure_path_matches_open_file(path, allowed_root, &before)?;

    let hard_read_limit = max_file_bytes.min(remaining_total_bytes);
    let mut reader = BufReader::new(file);
    let preview_limit = usize::try_from(preview_limit).unwrap_or(usize::MAX);
    let mut preview = Vec::with_capacity(preview_limit.min(16 * 1024));
    let mut buffer = [0_u8; 16 * 1024];
    let mut total_bytes = 0_u64;
    let mut line_count = 0_u64;
    let mut last_byte = None;
    let mut hasher = Sha256::new();

    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }

        if total_bytes.saturating_add(count as u64) > hard_read_limit {
            return Err(format!(
                "File changed while scanning and exceeded the {} MiB read limit.",
                hard_read_limit / (1024 * 1024)
            ));
        }
        hasher.update(&buffer[..count]);
        line_count = line_count.saturating_add(
            buffer[..count]
                .iter()
                .filter(|byte| **byte == b'\n')
                .count() as u64,
        );
        last_byte = buffer.get(count - 1).copied();
        total_bytes = total_bytes.saturating_add(count as u64);
        if preview.len() < preview_limit {
            let remaining = preview_limit - preview.len();
            preview.extend_from_slice(&buffer[..count.min(remaining)]);
        }
    }

    if total_bytes > 0 && last_byte != Some(b'\n') {
        line_count = line_count.saturating_add(1);
    }

    let after = reader
        .get_ref()
        .metadata()
        .map_err(|error| format!("Unable to recheck the open Harness file: {error}"))?;
    if !same_open_file_revision(&before, &after) || after.len() != total_bytes {
        return Err("File changed while it was being scanned.".to_string());
    }
    ensure_path_matches_open_file(path, allowed_root, &after)?;

    Ok(ScannedFileContent {
        preview: String::from_utf8_lossy(&preview).into_owned(),
        truncated: total_bytes > preview_limit as u64,
        content_hash: hex::encode(hasher.finalize()),
        line_count,
        size_bytes: total_bytes,
        modified_at: after.modified().ok(),
        metadata: after,
    })
}

fn ensure_path_matches_open_file(
    path: &Path,
    allowed_root: &Path,
    open_metadata: &fs::Metadata,
) -> Result<(), String> {
    let current_path = path
        .canonicalize()
        .map_err(|error| format!("Unable to re-resolve the Harness file path: {error}"))?;
    if !current_path.starts_with(allowed_root) {
        return Err(format!(
            "Resolved outside authorized root {} and was skipped.",
            allowed_root.to_string_lossy()
        ));
    }
    if current_path != path {
        return Err("File changed while it was being scanned.".to_string());
    }
    let path_metadata = fs::metadata(&current_path)
        .map_err(|error| format!("Unable to recheck the Harness file path: {error}"))?;
    if same_open_file_revision(open_metadata, &path_metadata) {
        Ok(())
    } else {
        Err("File changed while it was being scanned.".to_string())
    }
}

fn same_open_file_revision(before: &fs::Metadata, after: &fs::Metadata) -> bool {
    before.is_file()
        && after.is_file()
        && before.len() == after.len()
        && before.modified().ok() == after.modified().ok()
        && {
            #[cfg(unix)]
            {
                before.dev() == after.dev() && before.ino() == after.ino()
            }
            #[cfg(not(unix))]
            {
                true
            }
        }
}

fn quality_warnings(artifacts: &[HarnessArtifact]) -> Vec<HarnessWarning> {
    let mut guidance_line_review = Vec::new();
    let mut skill_description_missing = Vec::new();
    let mut empty_definition = Vec::new();
    let mut preview_truncated = Vec::new();
    let mut codex_project_instruction_budget = Vec::new();

    for artifact in artifacts {
        if matches!(
            artifact.kind,
            HarnessKind::Instructions | HarnessKind::Rule | HarnessKind::Skill | HarnessKind::Agent
        ) && artifact.line_count > GUIDANCE_LINE_REVIEW_THRESHOLD
        {
            guidance_line_review.push(artifact.id.clone());
        }
        if artifact.kind == HarnessKind::Skill
            && Path::new(&artifact.path)
                .file_name()
                .is_some_and(|name| name == "SKILL.md")
            && artifact
                .description
                .as_deref()
                .is_none_or(|description| description.trim().is_empty())
        {
            skill_description_missing.push(artifact.id.clone());
        }
        if artifact.kind != HarnessKind::Memory && artifact.size_bytes == 0 {
            empty_definition.push(artifact.id.clone());
        }
        if artifact.truncated {
            preview_truncated.push(artifact.id.clone());
        }
        if artifact.provider == HarnessProvider::Codex
            && artifact.kind == HarnessKind::Instructions
            && matches!(artifact.scope, HarnessScope::Repo | HarnessScope::Nested)
            && artifact.resolution == ResolutionState::Effective
            && artifact.size_bytes >= CODEX_PROJECT_INSTRUCTION_BUDGET_BYTES
        {
            codex_project_instruction_budget.push(artifact.id.clone());
        }
    }

    let mut warnings = Vec::new();
    push_grouped_warning(
        &mut warnings,
        "quality:guidance-line-review",
        WarningSeverity::Info,
        "Long guidance files may need review",
        "Guidance files over 200 lines deserve a focused maintainability review. The 200-line threshold is a Harness Lens maintainability heuristic, not a performance or success-rate conclusion.",
        guidance_line_review,
    );
    push_grouped_warning(
        &mut warnings,
        "quality:skill-description-missing",
        WarningSeverity::Info,
        "Skill description is missing",
        "These Skill definitions do not declare a non-empty frontmatter description.",
        skill_description_missing,
    );
    push_grouped_warning(
        &mut warnings,
        "quality:empty-definition",
        WarningSeverity::Warning,
        "Harness definition is empty",
        "These non-Memory Harness definitions are empty (0 bytes).",
        empty_definition,
    );
    push_grouped_warning(
        &mut warnings,
        "quality:preview-truncated",
        WarningSeverity::Info,
        "Harness preview is truncated",
        "Only a bounded preview is available for these files; the full files were still streamed once to compute their line counts and content hashes.",
        preview_truncated,
    );
    push_grouped_warning(
        &mut warnings,
        "quality:codex-project-instruction-budget",
        WarningSeverity::Warning,
        "Codex project instruction budget reached",
        "These repository or nested Codex instruction files are at least 32 KiB. A single file at this size reaches Codex's default 32 KiB combined project instruction limit.",
        codex_project_instruction_budget,
    );
    warnings
}

fn push_grouped_warning(
    warnings: &mut Vec<HarnessWarning>,
    id: &str,
    severity: WarningSeverity,
    title: &str,
    detail: &str,
    mut artifact_ids: Vec<String>,
) {
    if artifact_ids.is_empty() {
        return;
    }
    artifact_ids.sort();
    artifact_ids.dedup();
    warnings.push(HarnessWarning {
        id: id.to_string(),
        severity,
        title: title.to_string(),
        detail: detail.to_string(),
        artifact_ids,
    });
}

fn warning_severity_order(severity: &WarningSeverity) -> u8 {
    match severity {
        WarningSeverity::Error => 0,
        WarningSeverity::Warning => 1,
        WarningSeverity::Info => 2,
    }
}

fn incomplete_scan_warning(diagnostics: &[String]) -> HarnessWarning {
    const MAX_DETAILS: usize = 8;

    let mut details = diagnostics
        .iter()
        .take(MAX_DETAILS)
        .cloned()
        .collect::<Vec<_>>();
    if diagnostics.len() > MAX_DETAILS {
        details.push(format!(
            "{} additional scan issue(s) were omitted from this summary.",
            diagnostics.len() - MAX_DETAILS
        ));
    }

    HarnessWarning {
        id: "scan-incomplete".to_string(),
        severity: WarningSeverity::Warning,
        title: "Harness scan was incomplete".to_string(),
        detail: details.join(" "),
        artifact_ids: Vec::new(),
    }
}

fn record_scan_diagnostic(diagnostics: &mut Vec<String>, diagnostic: String) {
    if diagnostics.len() < MAX_SCAN_DIAGNOSTICS {
        diagnostics.push(diagnostic);
    } else if diagnostics.len() == MAX_SCAN_DIAGNOSTICS {
        diagnostics.push(
            "Further scan diagnostics were suppressed to stay within the reporting budget."
                .to_string(),
        );
    }
}

fn parse_skill_frontmatter(content: &str) -> (Option<String>, Option<String>) {
    let mut content_lines = content.lines();
    if content_lines.next() != Some("---") {
        return (None, None);
    }
    let mut block = Vec::new();
    let mut closed = false;
    for line in content_lines {
        if line == "---" {
            closed = true;
            break;
        }
        block.push(line);
    }
    if !closed {
        return (None, None);
    }
    let name = frontmatter_scalar(&block, "name", false);
    let description = frontmatter_scalar(&block, "description", true);
    (name, description)
}

fn frontmatter_scalar(lines: &[&str], key: &str, normalize_whitespace: bool) -> Option<String> {
    let prefix = format!("{key}:");
    let (index, raw_value) = lines.iter().enumerate().find_map(|(index, line)| {
        line.strip_prefix(&prefix)
            .map(|value| (index, value.trim()))
    })?;

    let indicator = raw_value.split_whitespace().next().unwrap_or_default();
    let value = if is_yaml_block_scalar_indicator(indicator) {
        lines[index + 1..]
            .iter()
            .take_while(|line| line.trim().is_empty() || line.starts_with([' ', '\t']))
            .map(|line| line.trim())
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        raw_value.trim_matches(['\"', '\'']).to_string()
    };
    let value = if normalize_whitespace {
        value.split_whitespace().collect::<Vec<_>>().join(" ")
    } else {
        value.trim().to_string()
    };

    (!value.is_empty()).then_some(value)
}

fn is_yaml_block_scalar_indicator(value: &str) -> bool {
    let mut characters = value.chars();
    matches!(characters.next(), Some('|' | '>'))
        && characters.all(|character| character.is_ascii_digit() || matches!(character, '+' | '-'))
}

fn annotate_duplicates_and_counterpart_differences(
    artifacts: &mut [HarnessArtifact],
    repo_root: &Path,
) -> Vec<HarnessWarning> {
    let mut warnings = Vec::new();
    let mut by_hash: HashMap<(String, String), Vec<usize>> = HashMap::new();
    let mut by_name: HashMap<(String, String, String, String), Vec<usize>> = HashMap::new();
    for (index, artifact) in artifacts.iter().enumerate() {
        let kind = format!("{:?}", artifact.kind);
        let scope = format!("{:?}", artifact.scope);
        let scope_anchor = counterpart_scope_anchor(artifact, repo_root);
        by_hash
            .entry((kind.clone(), artifact.content_hash.clone()))
            .or_default()
            .push(index);
        by_name
            .entry((scope, scope_anchor, kind, artifact.name.to_lowercase()))
            .or_default()
            .push(index);
    }

    for ((_, hash), indexes) in by_hash.into_iter().filter(|(_, indexes)| indexes.len() > 1) {
        let group = format!("duplicate:{}", &hash[..12]);
        let ids = indexes
            .iter()
            .map(|index| artifacts[*index].id.clone())
            .collect::<Vec<_>>();
        for index in indexes {
            artifacts[index].duplicate_group_id = Some(group.clone());
        }
        warnings.push(HarnessWarning {
            id: group,
            severity: WarningSeverity::Info,
            title: "Duplicate Harness content".to_string(),
            detail: "Multiple discovered items have identical content.".to_string(),
            artifact_ids: ids,
        });
    }

    for ((scope, scope_anchor, kind, name), indexes) in
        by_name.into_iter().filter(|(_, indexes)| indexes.len() > 1)
    {
        let peers = indexes
            .iter()
            .map(|index| {
                (
                    artifacts[*index].id.clone(),
                    artifacts[*index].provider.clone(),
                    artifacts[*index].content_hash.clone(),
                )
            })
            .collect::<Vec<_>>();
        let mut ids = Vec::new();
        for index in indexes {
            let provider = artifacts[index].provider.clone();
            let content_hash = artifacts[index].content_hash.clone();
            let counterpart_id = peers
                .iter()
                .find(|(_, candidate_provider, candidate_hash)| {
                    *candidate_provider != provider && *candidate_hash != content_hash
                })
                .map(|(id, _, _)| id.clone());
            if counterpart_id.is_some() {
                ids.push(artifacts[index].id.clone());
                artifacts[index].counterpart_id = counterpart_id;
            }
        }
        if ids.is_empty() {
            continue;
        }
        let anchor_hash = hex::encode(Sha256::digest(scope_anchor.as_bytes()));
        warnings.push(HarnessWarning {
            id: format!(
                "counterpart-difference:{scope}:{}:{kind}:{name}",
                &anchor_hash[..12]
            ),
            severity: WarningSeverity::Info,
            title: format!("Same-name content differs: {name}"),
            detail: "Same-name Harness items in the same project layer have different content across providers."
                .to_string(),
            artifact_ids: ids,
        });
    }

    warnings
}

fn counterpart_scope_anchor(artifact: &HarnessArtifact, repo_root: &Path) -> String {
    match artifact.scope {
        HarnessScope::User => "user".to_string(),
        HarnessScope::Repo => repo_root.to_string_lossy().into_owned(),
        HarnessScope::Nested => project_layer_anchor(Path::new(&artifact.path), repo_root)
            .to_string_lossy()
            .into_owned(),
        HarnessScope::Worktree => worktree_anchor(Path::new(&artifact.path))
            .to_string_lossy()
            .into_owned(),
    }
}

fn project_layer_anchor(path: &Path, fallback: &Path) -> PathBuf {
    for ancestor in path.ancestors() {
        let is_harness_directory = ancestor
            .file_name()
            .is_some_and(|name| name == ".codex" || name == ".claude" || name == ".agents");
        if is_harness_directory {
            return ancestor.parent().unwrap_or(fallback).to_path_buf();
        }
    }
    path.parent().unwrap_or(fallback).to_path_buf()
}

fn worktree_anchor(path: &Path) -> PathBuf {
    path.ancestors()
        .find(|ancestor| ancestor.file_name().is_some_and(|name| name == "memory"))
        .and_then(Path::parent)
        .unwrap_or_else(|| path.parent().unwrap_or(path))
        .to_path_buf()
}

fn directory_chain(root: &Path, workspace: &Path) -> Vec<PathBuf> {
    if !workspace.starts_with(root) {
        return vec![workspace.to_path_buf()];
    }
    let mut chain = vec![root.to_path_buf()];
    let relative = workspace.strip_prefix(root).unwrap_or(Path::new(""));
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        chain.push(current.clone());
    }
    chain
}

fn git_root(workspace: &Path) -> Option<PathBuf> {
    command_output(workspace, &["rev-parse", "--show-toplevel"]).map(PathBuf::from)
}

fn git_branch(workspace: &Path) -> Option<String> {
    command_output(workspace, &["branch", "--show-current"]).filter(|branch| !branch.is_empty())
}

fn command_output(workspace: &Path, arguments: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(arguments)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn is_non_empty_file(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
}

fn not_hidden_or_root(entry: &DirEntry) -> bool {
    entry.depth() == 0 || !entry.file_name().to_string_lossy().starts_with('.')
}

fn system_time_to_rfc3339(time: SystemTime) -> String {
    let datetime: DateTime<Utc> = time.into();
    datetime.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, fs, fs::File, path::Path, process::Command};

    use tempfile::tempdir;

    use super::{
        artifact_id, deduplicate_candidates, ensure_path_matches_open_file, read_preview_and_hash,
        scan, Candidate, CODEX_PROJECT_INSTRUCTION_BUDGET_BYTES, MAX_CONTENT_BYTES, MAX_FILE_BYTES,
    };
    use crate::{
        memory_edit::MAX_MEMORY_BYTES,
        model::{HarnessKind, HarnessProvider, HarnessScope, ResolutionState, WarningSeverity},
    };

    fn initialize_git_repository(path: &Path) {
        let status = Command::new("git")
            .arg("init")
            .arg("--quiet")
            .arg(path)
            .status()
            .expect("git must be available for scanner tests");
        assert!(status.success());
    }

    fn write_skill(path: &Path, name: &str, description: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            format!("---\nname: {name}\ndescription: {description}\n---\nBody"),
        )
        .unwrap();
    }

    fn physical_lines(count: usize) -> String {
        "line\n".repeat(count)
    }

    #[test]
    fn scans_preferred_instructions_skills_and_redacts_secrets() {
        let home = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        fs::create_dir_all(home.path().join(".codex")).unwrap();
        fs::write(home.path().join(".codex/AGENTS.md"), "base").unwrap();
        fs::write(home.path().join(".codex/AGENTS.override.md"), "override").unwrap();
        fs::write(
            home.path().join(".codex/config.toml"),
            "api_key = \"must-not-leak\"\nmodel = \"safe\"",
        )
        .unwrap();
        write_skill(
            &workspace.path().join(".agents/skills/verify/SKILL.md"),
            "verify",
            "api_key = must-not-leak-from-description",
        );

        let snapshot = scan(workspace.path(), home.path()).unwrap();

        let override_instructions = snapshot
            .artifacts
            .iter()
            .find(|item| item.path.ends_with("AGENTS.override.md"))
            .unwrap();
        let base_instructions = snapshot
            .artifacts
            .iter()
            .find(|item| item.path.ends_with(".codex/AGENTS.md"))
            .unwrap();
        assert_eq!(override_instructions.resolution, ResolutionState::Effective);
        assert_eq!(base_instructions.resolution, ResolutionState::Shadowed);

        let skill = snapshot
            .artifacts
            .iter()
            .find(|item| item.kind == HarnessKind::Skill && item.name == "verify")
            .unwrap();
        assert!(!skill
            .description
            .as_deref()
            .unwrap()
            .contains("must-not-leak-from-description"));

        let config = snapshot
            .artifacts
            .iter()
            .find(|item| item.path.ends_with("config.toml"))
            .unwrap();
        assert!(!config.content.as_deref().unwrap().contains("must-not-leak"));
    }

    #[test]
    fn marks_base_instructions_effective_when_no_override_exists() {
        let home = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        fs::create_dir_all(home.path().join(".codex")).unwrap();
        fs::write(home.path().join(".codex/AGENTS.md"), "base").unwrap();

        let snapshot = scan(workspace.path(), home.path()).unwrap();
        let instructions = snapshot
            .artifacts
            .iter()
            .find(|item| item.path.ends_with(".codex/AGENTS.md"))
            .unwrap();

        assert_eq!(instructions.resolution, ResolutionState::Effective);
    }

    #[test]
    fn discovers_repo_codex_content_and_effective_ancestor_skills() {
        let home = tempdir().unwrap();
        let repository = tempdir().unwrap();
        initialize_git_repository(repository.path());
        let workspace = repository.path().join("packages/app");
        fs::create_dir_all(&workspace).unwrap();

        write_skill(
            &repository.path().join(".codex/skills/codex-only/SKILL.md"),
            "codex-only",
            "Codex repository skill.",
        );
        write_skill(
            &repository.path().join(".agents/skills/root-skill/SKILL.md"),
            "root-skill",
            "Root ancestor skill.",
        );
        write_skill(
            &repository
                .path()
                .join("packages/.agents/skills/package-skill/SKILL.md"),
            "package-skill",
            "Nested ancestor skill.",
        );
        fs::create_dir_all(repository.path().join(".codex/rules")).unwrap();
        fs::write(
            repository.path().join(".codex/rules/safety.rules"),
            "prefix_rule(pattern=[\"git\"], decision=\"allow\")",
        )
        .unwrap();
        fs::create_dir_all(repository.path().join(".codex/hooks")).unwrap();
        fs::write(
            repository.path().join(".codex/hooks/preflight.sh"),
            "#!/bin/sh\nexit 0",
        )
        .unwrap();
        fs::write(repository.path().join(".codex/hooks.json"), "{}").unwrap();
        fs::create_dir_all(repository.path().join(".codex/memories")).unwrap();
        fs::write(
            repository.path().join(".codex/memories/MEMORY.md"),
            "Project memory",
        )
        .unwrap();

        let snapshot = scan(&workspace, home.path()).unwrap();
        let resolution_for = |suffix: &str| {
            snapshot
                .artifacts
                .iter()
                .find(|item| item.path.ends_with(suffix))
                .unwrap_or_else(|| panic!("missing artifact: {suffix}"))
                .resolution
                .clone()
        };

        assert_eq!(
            resolution_for(".codex/skills/codex-only/SKILL.md"),
            ResolutionState::Defined
        );
        assert_eq!(
            resolution_for(".codex/rules/safety.rules"),
            ResolutionState::Defined
        );
        assert_eq!(
            resolution_for(".codex/hooks.json"),
            ResolutionState::Unknown
        );
        assert_eq!(
            resolution_for(".codex/hooks/preflight.sh"),
            ResolutionState::Unknown
        );
        assert_eq!(
            resolution_for(".codex/memories/MEMORY.md"),
            ResolutionState::Defined
        );
        assert_eq!(
            resolution_for(".agents/skills/root-skill/SKILL.md"),
            ResolutionState::Effective
        );
        assert_eq!(
            resolution_for("packages/.agents/skills/package-skill/SKILL.md"),
            ResolutionState::Effective
        );
    }

    #[test]
    fn discovers_project_harness_content_at_each_workspace_ancestor() {
        let home = tempdir().unwrap();
        let repository = tempdir().unwrap();
        initialize_git_repository(repository.path());
        let package = repository.path().join("packages");
        let workspace = package.join("app");
        fs::create_dir_all(&workspace).unwrap();

        fs::create_dir_all(repository.path().join(".codex/rules")).unwrap();
        fs::write(
            repository.path().join(".codex/rules/root.rules"),
            "allow root",
        )
        .unwrap();
        write_skill(
            &package.join(".codex/skills/package-codex/SKILL.md"),
            "package-codex",
            "Package Codex skill.",
        );
        fs::create_dir_all(workspace.join(".claude/rules")).unwrap();
        fs::write(
            workspace.join(".claude/rules/app.md"),
            "Use app-level rules.",
        )
        .unwrap();
        fs::create_dir_all(workspace.join(".claude/memory")).unwrap();
        fs::write(
            workspace.join(".claude/memory/LOCAL.md"),
            "App-level memory.",
        )
        .unwrap();

        let snapshot = scan(&workspace, home.path()).unwrap();
        let artifact_for = |suffix: &str| {
            snapshot
                .artifacts
                .iter()
                .find(|item| item.path.ends_with(suffix))
                .unwrap_or_else(|| panic!("missing artifact: {suffix}"))
        };

        assert_eq!(
            artifact_for(".codex/rules/root.rules").scope,
            HarnessScope::Repo
        );
        assert_eq!(
            artifact_for("packages/.codex/skills/package-codex/SKILL.md").scope,
            HarnessScope::Nested
        );
        assert_eq!(
            artifact_for("packages/app/.claude/rules/app.md").scope,
            HarnessScope::Nested
        );
        assert_eq!(
            artifact_for("packages/app/.claude/memory/LOCAL.md").scope,
            HarnessScope::Nested
        );
        assert_eq!(
            snapshot
                .artifacts
                .iter()
                .filter(|item| item.path.ends_with(".claude/memory/LOCAL.md"))
                .count(),
            1
        );
    }

    #[test]
    fn discovers_user_maintained_codex_memory_extensions_only() {
        let home = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let memories = home.path().join(".codex/memories");
        fs::create_dir_all(memories.join("extensions/ad_hoc/notes")).unwrap();
        fs::create_dir_all(memories.join("rollout_summaries")).unwrap();
        fs::create_dir_all(memories.join("skills/example")).unwrap();
        fs::write(memories.join("MEMORY.md"), "Memory registry").unwrap();
        fs::write(memories.join("memory_summary.md"), "Memory summary").unwrap();
        fs::write(
            memories.join("extensions/ad_hoc/preferences.md"),
            "Preference extension",
        )
        .unwrap();
        fs::write(
            memories.join("extensions/ad_hoc/notes/project.md"),
            "Project note",
        )
        .unwrap();
        fs::write(
            memories.join("rollout_summaries/session.md"),
            "Runtime evidence",
        )
        .unwrap();
        fs::write(
            memories.join("skills/example/SKILL.md"),
            "Memory-related skill",
        )
        .unwrap();

        let snapshot = scan(workspace.path(), home.path()).unwrap();
        let memory_paths = snapshot
            .artifacts
            .iter()
            .filter(|item| item.kind == HarnessKind::Memory)
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(memory_paths.len(), 4);
        assert!(memory_paths
            .iter()
            .any(|path| path.ends_with("memories/MEMORY.md")));
        assert!(memory_paths
            .iter()
            .any(|path| path.ends_with("memories/memory_summary.md")));
        assert!(memory_paths
            .iter()
            .any(|path| path.ends_with("extensions/ad_hoc/preferences.md")));
        assert!(memory_paths
            .iter()
            .any(|path| path.ends_with("extensions/ad_hoc/notes/project.md")));
        assert!(!memory_paths
            .iter()
            .any(|path| path.contains("rollout_summaries")));
        assert!(!memory_paths
            .iter()
            .any(|path| path.contains("memories/skills")));
        assert!(snapshot
            .artifacts
            .iter()
            .filter(|item| item.kind == HarnessKind::Memory)
            .all(|item| item.scope == HarnessScope::User && item.content.is_none()));
    }

    #[test]
    fn marks_memory_above_editor_limit_as_external_only() {
        let home = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let memories = workspace.path().join(".codex/memories");
        fs::create_dir_all(&memories).unwrap();
        fs::write(
            memories.join("LARGE.md"),
            vec![b'x'; MAX_MEMORY_BYTES as usize + 1],
        )
        .unwrap();

        let snapshot = scan(workspace.path(), home.path()).unwrap();
        let memory = snapshot
            .artifacts
            .iter()
            .find(|artifact| artifact.path.ends_with(".codex/memories/LARGE.md"))
            .expect("large Memory artifact");

        assert!(!memory.editable);
        assert!(memory
            .editability_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("opened externally")));
    }

    #[test]
    fn discovers_claude_rules_commands_and_memory_as_defined_metadata() {
        let home = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let workspace_key = super::claude_project_key(&workspace.path().canonicalize().unwrap());
        let project_memory = home
            .path()
            .join(".claude/projects")
            .join(&workspace_key)
            .join("memory");
        fs::create_dir_all(&project_memory).unwrap();
        fs::write(project_memory.join("MEMORY.md"), "Private user memory").unwrap();
        fs::create_dir_all(workspace.path().join(".claude/rules")).unwrap();
        fs::write(
            workspace.path().join(".claude/rules/style.md"),
            "Use the project style.",
        )
        .unwrap();
        fs::create_dir_all(workspace.path().join(".claude/commands")).unwrap();
        fs::write(
            workspace.path().join(".claude/commands/review.md"),
            "Review the current diff.",
        )
        .unwrap();
        fs::create_dir_all(workspace.path().join(".claude/memory")).unwrap();
        fs::write(
            workspace.path().join(".claude/memory/PROJECT.md"),
            "Private repository memory",
        )
        .unwrap();

        let snapshot = scan(workspace.path(), home.path()).unwrap();
        let artifact_for = |suffix: &str| {
            snapshot
                .artifacts
                .iter()
                .find(|item| item.path.ends_with(suffix))
                .unwrap_or_else(|| panic!("missing artifact: {suffix}"))
        };
        let user_memory = snapshot
            .artifacts
            .iter()
            .find(|item| {
                item.path.ends_with("/memory/MEMORY.md") && item.path.contains(&workspace_key)
            })
            .expect("missing workspace-specific Claude memory");
        let repo_memory = artifact_for(".claude/memory/PROJECT.md");
        let rule = artifact_for(".claude/rules/style.md");
        let command = artifact_for(".claude/commands/review.md");

        assert_eq!(user_memory.resolution, ResolutionState::Defined);
        assert_eq!(user_memory.scope, HarnessScope::Worktree);
        assert_eq!(repo_memory.resolution, ResolutionState::Defined);
        assert!(user_memory.content.is_none());
        assert!(repo_memory.content.is_none());
        assert_eq!(rule.resolution, ResolutionState::Defined);
        assert_eq!(command.resolution, ResolutionState::Defined);
        assert_eq!(command.name, "review");
    }

    #[test]
    fn keeps_artifact_id_stable_and_hashes_beyond_preview_limit() {
        let home = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let agents = workspace.path().join(".codex/agents");
        fs::create_dir_all(&agents).unwrap();

        let shared_prefix = vec![b'a'; MAX_CONTENT_BYTES as usize];
        let mut first_content = shared_prefix.clone();
        first_content.extend_from_slice(b"first-tail");
        let mut second_content = shared_prefix;
        second_content.extend_from_slice(b"second-tail");
        let first_path = agents.join("first.toml");
        let second_path = agents.join("second.toml");
        fs::write(&first_path, &first_content).unwrap();
        fs::write(&second_path, &second_content).unwrap();

        let first_snapshot = scan(workspace.path(), home.path()).unwrap();
        let first = first_snapshot
            .artifacts
            .iter()
            .find(|item| item.path.ends_with(".codex/agents/first.toml"))
            .unwrap();
        let second = first_snapshot
            .artifacts
            .iter()
            .find(|item| item.path.ends_with(".codex/agents/second.toml"))
            .unwrap();
        let stable_id = first.id.clone();
        let original_hash = first.content_hash.clone();

        assert!(first.truncated && second.truncated);
        assert_eq!(first.content, second.content);
        assert_ne!(first.content_hash, second.content_hash);
        assert!(first.duplicate_group_id.is_none());
        assert!(second.duplicate_group_id.is_none());
        let truncated_warning = first_snapshot
            .warnings
            .iter()
            .find(|warning| warning.id == "quality:preview-truncated")
            .expect("truncated previews are grouped into one warning");
        let mut expected_ids = vec![first.id.clone(), second.id.clone()];
        expected_ids.sort();
        assert_eq!(truncated_warning.artifact_ids, expected_ids);
        assert_eq!(
            first_snapshot
                .warnings
                .iter()
                .filter(|warning| warning.id == "quality:preview-truncated")
                .count(),
            1
        );

        fs::write(&first_path, b"edited content").unwrap();
        let edited_snapshot = scan(workspace.path(), home.path()).unwrap();
        let edited = edited_snapshot
            .artifacts
            .iter()
            .find(|item| item.path.ends_with(".codex/agents/first.toml"))
            .unwrap();

        assert_eq!(edited.id, stable_id);
        assert_ne!(edited.content_hash, original_hash);
    }

    #[test]
    fn counts_full_physical_lines_at_preview_boundaries() {
        let directory = tempdir().unwrap();
        let allowed_root = directory.path().canonicalize().unwrap();
        let path = allowed_root.join("lines.txt");

        fs::write(&path, b"").unwrap();
        let scanned = read_preview_and_hash(&path, &allowed_root, 4, 16, 16).unwrap();
        assert!(!scanned.truncated);
        assert_eq!(scanned.line_count, 0);
        assert_eq!(scanned.size_bytes, 0);

        fs::write(&path, b"a\nb\n").unwrap();
        let scanned = read_preview_and_hash(&path, &allowed_root, 4, 16, 16).unwrap();
        assert_eq!(scanned.preview, "a\nb\n");
        assert!(!scanned.truncated);
        assert_eq!(scanned.line_count, 2);
        assert_eq!(scanned.size_bytes, 4);

        fs::write(&path, b"a\nb\nc").unwrap();
        let scanned = read_preview_and_hash(&path, &allowed_root, 4, 16, 16).unwrap();
        assert_eq!(scanned.preview, "a\nb\n");
        assert!(scanned.truncated);
        assert_eq!(scanned.line_count, 3);
        assert_eq!(scanned.size_bytes, 5);

        fs::write(&path, b"a\nb").unwrap();
        let scanned = read_preview_and_hash(&path, &allowed_root, 4, 16, 16).unwrap();
        assert!(!scanned.truncated);
        assert_eq!(scanned.line_count, 2);
        assert_eq!(scanned.size_bytes, 3);
    }

    #[cfg(unix)]
    #[test]
    fn detects_atomic_path_replacement_after_opening_a_scan_source() {
        let directory = tempdir().unwrap();
        let allowed_root = directory.path().canonicalize().unwrap();
        let path = allowed_root.join("source.md");
        let replacement = allowed_root.join("replacement.md");
        fs::write(&path, b"before").unwrap();
        fs::write(&replacement, b"after!").unwrap();
        let open_file = File::open(&path).unwrap();
        let open_metadata = open_file.metadata().unwrap();

        fs::rename(&replacement, &path).unwrap();

        assert_eq!(
            ensure_path_matches_open_file(&path, &allowed_root, &open_metadata),
            Err("File changed while it was being scanned.".to_string())
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_scan_source_retargeted_outside_its_authorized_root() {
        let directory = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let allowed_root = directory.path().canonicalize().unwrap();
        let outside_root = outside.path().canonicalize().unwrap();
        let path = allowed_root.join("source.md");
        let outside_path = outside_root.join("outside.md");
        fs::write(&outside_path, b"private").unwrap();
        std::os::unix::fs::symlink(&outside_path, &path).unwrap();

        let error = read_preview_and_hash(&path, &allowed_root, 16, 16, 16).unwrap_err();

        assert!(error.contains("Resolved outside authorized root"));
        assert!(!error.contains(outside_path.to_string_lossy().as_ref()));
    }

    #[test]
    fn groups_guidance_over_200_lines_and_excludes_other_kinds() {
        let home = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        fs::write(workspace.path().join("AGENTS.md"), physical_lines(201)).unwrap();
        fs::create_dir_all(workspace.path().join(".codex/rules")).unwrap();
        fs::write(
            workspace.path().join(".codex/rules/boundary.rules"),
            physical_lines(200),
        )
        .unwrap();
        fs::write(
            workspace.path().join(".codex/rules/long.rules"),
            physical_lines(201),
        )
        .unwrap();
        fs::create_dir_all(workspace.path().join(".codex/agents")).unwrap();
        fs::write(
            workspace.path().join(".codex/agents/long.md"),
            physical_lines(201),
        )
        .unwrap();
        fs::create_dir_all(workspace.path().join(".codex/skills/long")).unwrap();
        fs::write(
            workspace.path().join(".codex/skills/long/SKILL.md"),
            format!(
                "---\nname: long\ndescription: Long skill.\n---\n{}",
                physical_lines(197)
            ),
        )
        .unwrap();
        fs::create_dir_all(workspace.path().join(".codex/hooks")).unwrap();
        fs::write(
            workspace.path().join(".codex/hooks/long.sh"),
            physical_lines(201),
        )
        .unwrap();
        fs::write(
            workspace.path().join(".codex/config.toml"),
            physical_lines(201),
        )
        .unwrap();
        fs::create_dir_all(workspace.path().join(".codex/memories")).unwrap();
        fs::write(
            workspace.path().join(".codex/memories/long.md"),
            physical_lines(201),
        )
        .unwrap();

        let first_snapshot = scan(workspace.path(), home.path()).unwrap();
        let second_snapshot = scan(workspace.path(), home.path()).unwrap();
        let warning = first_snapshot
            .warnings
            .iter()
            .find(|warning| warning.id == "quality:guidance-line-review")
            .expect("long guidance warning");
        let mut expected_ids = first_snapshot
            .artifacts
            .iter()
            .filter(|artifact| {
                artifact.path.ends_with("AGENTS.md")
                    || artifact.path.ends_with(".codex/rules/long.rules")
                    || artifact.path.ends_with(".codex/agents/long.md")
                    || artifact.path.ends_with(".codex/skills/long/SKILL.md")
            })
            .map(|artifact| artifact.id.clone())
            .collect::<Vec<_>>();
        expected_ids.sort();

        assert_eq!(warning.artifact_ids, expected_ids);
        assert!(warning
            .detail
            .contains("Harness Lens maintainability heuristic"));
        assert!(warning
            .detail
            .contains("not a performance or success-rate conclusion"));
        assert!(!warning.artifact_ids.iter().any(|id| {
            first_snapshot.artifacts.iter().any(|artifact| {
                artifact.id == *id
                    && (artifact.path.ends_with("boundary.rules")
                        || artifact.kind == HarnessKind::Hook
                        || artifact.kind == HarnessKind::Config
                        || artifact.kind == HarnessKind::Memory)
            })
        }));
        let serialized = serde_json::to_value(
            first_snapshot
                .artifacts
                .iter()
                .find(|artifact| artifact.path.ends_with(".codex/agents/long.md"))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(serialized["lineCount"], 201);
        assert!(serialized.get("line_count").is_none());

        let quality_groups = |snapshot: &crate::model::HarnessSnapshot| {
            snapshot
                .warnings
                .iter()
                .filter(|warning| warning.id.starts_with("quality:"))
                .map(|warning| (warning.id.clone(), warning.artifact_ids.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            quality_groups(&first_snapshot),
            quality_groups(&second_snapshot)
        );
    }

    #[test]
    fn groups_skills_without_non_empty_descriptions() {
        let home = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let skills = workspace.path().join(".agents/skills");
        write_skill(
            &skills.join("described/SKILL.md"),
            "described",
            "Useful description.",
        );
        write_skill(&skills.join("blank/SKILL.md"), "blank", "");
        fs::create_dir_all(skills.join("missing")).unwrap();
        fs::write(
            skills.join("missing/SKILL.md"),
            "---\nname: missing\n---\nBody",
        )
        .unwrap();
        fs::create_dir_all(skills.join("folded")).unwrap();
        fs::write(
            skills.join("folded/SKILL.md"),
            "---\nname: folded\ndescription: >-\n  Reviews changes and\n  reports evidence.\n---\nBody",
        )
        .unwrap();
        fs::create_dir_all(skills.join("literal")).unwrap();
        fs::write(
            skills.join("literal/SKILL.md"),
            "---\nname: literal\ndescription: |\n  Runs focused checks.\n  Keeps results concise.\n---\nBody",
        )
        .unwrap();
        fs::create_dir_all(workspace.path().join(".claude/commands")).unwrap();
        fs::write(
            workspace.path().join(".claude/commands/review.md"),
            "Review the current diff.",
        )
        .unwrap();

        let snapshot = scan(workspace.path(), home.path()).unwrap();
        let warning = snapshot
            .warnings
            .iter()
            .find(|warning| warning.id == "quality:skill-description-missing")
            .expect("missing description warning");
        let mut expected_ids = snapshot
            .artifacts
            .iter()
            .filter(|artifact| artifact.name == "blank" || artifact.name == "missing")
            .map(|artifact| artifact.id.clone())
            .collect::<Vec<_>>();
        expected_ids.sort();

        assert_eq!(warning.artifact_ids, expected_ids);
        assert_eq!(
            snapshot
                .artifacts
                .iter()
                .find(|artifact| artifact.name == "folded")
                .and_then(|artifact| artifact.description.as_deref()),
            Some("Reviews changes and reports evidence.")
        );
        assert_eq!(
            snapshot
                .artifacts
                .iter()
                .find(|artifact| artifact.name == "literal")
                .and_then(|artifact| artifact.description.as_deref()),
            Some("Runs focused checks. Keeps results concise.")
        );
        let legacy_command = snapshot
            .artifacts
            .iter()
            .find(|artifact| artifact.path.ends_with(".claude/commands/review.md"))
            .expect("legacy Claude command");
        assert!(!warning.artifact_ids.contains(&legacy_command.id));
        assert_eq!(
            snapshot
                .warnings
                .iter()
                .filter(|warning| warning.id == "quality:skill-description-missing")
                .count(),
            1
        );
    }

    #[test]
    fn groups_empty_non_memory_definitions() {
        let home = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        fs::create_dir_all(workspace.path().join(".codex/agents")).unwrap();
        fs::create_dir_all(workspace.path().join(".codex/memories")).unwrap();
        fs::write(workspace.path().join(".codex/config.toml"), b"").unwrap();
        fs::write(workspace.path().join(".codex/agents/empty.md"), b"").unwrap();
        fs::write(workspace.path().join(".codex/memories/empty.md"), b"").unwrap();

        let snapshot = scan(workspace.path(), home.path()).unwrap();
        let warning = snapshot
            .warnings
            .iter()
            .find(|warning| warning.id == "quality:empty-definition")
            .expect("empty definition warning");
        let mut expected_ids = snapshot
            .artifacts
            .iter()
            .filter(|artifact| artifact.size_bytes == 0 && artifact.kind != HarnessKind::Memory)
            .map(|artifact| artifact.id.clone())
            .collect::<Vec<_>>();
        expected_ids.sort();

        assert_eq!(warning.severity, WarningSeverity::Warning);
        assert_eq!(warning.artifact_ids, expected_ids);
        assert!(snapshot
            .artifacts
            .iter()
            .any(|artifact| artifact.kind == HarnessKind::Memory && artifact.size_bytes == 0));
    }

    #[test]
    fn groups_repo_and_nested_codex_instructions_at_32_kib() {
        let home = tempdir().unwrap();
        let repository = tempdir().unwrap();
        initialize_git_repository(repository.path());
        let package = repository.path().join("packages");
        let workspace = package.join("app");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(home.path().join(".codex")).unwrap();
        let at_budget = vec![b'x'; CODEX_PROJECT_INSTRUCTION_BUDGET_BYTES as usize];
        let below_budget = vec![b'x'; CODEX_PROJECT_INSTRUCTION_BUDGET_BYTES as usize - 1];
        fs::write(home.path().join(".codex/AGENTS.md"), &at_budget).unwrap();
        fs::write(repository.path().join("AGENTS.md"), &at_budget).unwrap();
        fs::write(package.join("AGENTS.md"), &at_budget).unwrap();
        fs::write(package.join("AGENTS.override.md"), &below_budget).unwrap();
        fs::write(workspace.join("AGENTS.md"), &at_budget).unwrap();
        fs::write(repository.path().join("CLAUDE.md"), &at_budget).unwrap();

        let snapshot = scan(&workspace, home.path()).unwrap();
        let warning = snapshot
            .warnings
            .iter()
            .find(|warning| warning.id == "quality:codex-project-instruction-budget")
            .expect("Codex project instruction budget warning");
        let mut expected_ids = snapshot
            .artifacts
            .iter()
            .filter(|artifact| {
                artifact.provider == HarnessProvider::Codex
                    && artifact.kind == HarnessKind::Instructions
                    && matches!(artifact.scope, HarnessScope::Repo | HarnessScope::Nested)
                    && artifact.resolution == ResolutionState::Effective
                    && artifact.size_bytes == CODEX_PROJECT_INSTRUCTION_BUDGET_BYTES
            })
            .map(|artifact| artifact.id.clone())
            .collect::<Vec<_>>();
        expected_ids.sort();

        assert_eq!(warning.severity, WarningSeverity::Warning);
        assert_eq!(warning.artifact_ids, expected_ids);
        assert_eq!(warning.artifact_ids.len(), 2);
        let shadowed_package_instructions = snapshot
            .artifacts
            .iter()
            .find(|artifact| artifact.path.ends_with("packages/AGENTS.md"))
            .expect("shadowed package instructions");
        assert_eq!(
            shadowed_package_instructions.resolution,
            ResolutionState::Shadowed
        );
        assert!(!warning
            .artifact_ids
            .contains(&shadowed_package_instructions.id));
        assert!(warning
            .detail
            .contains("default 32 KiB combined project instruction limit"));
        assert_eq!(
            snapshot
                .warnings
                .iter()
                .filter(|warning| warning.id == "quality:codex-project-instruction-budget")
                .count(),
            1
        );
    }

    #[test]
    fn deduplicates_repeated_candidates_for_the_same_source_identity() {
        let workspace = tempdir().unwrap();
        let candidate = Candidate {
            path: workspace.path().join(".codex/memories/project.md"),
            name: None,
            kind: HarnessKind::Memory,
            provider: HarnessProvider::Codex,
            scope: HarnessScope::Repo,
            resolution: ResolutionState::Defined,
            reason: "test candidate".to_string(),
            sensitive: true,
            metadata_only: true,
        };
        let expected_id = artifact_id(&candidate);
        let mut candidates = vec![candidate.clone(), candidate];

        deduplicate_candidates(&mut candidates);

        assert_eq!(candidates.len(), 1);
        assert_eq!(artifact_id(&candidates[0]), expected_id);
    }

    #[test]
    fn reports_same_scope_provider_difference_without_overwriting_resolution() {
        let home = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        write_skill(
            &workspace.path().join(".agents/skills/qa/SKILL.md"),
            "qa",
            "Shared QA skill.",
        );
        write_skill(
            &workspace.path().join(".claude/skills/qa/SKILL.md"),
            "qa",
            "Claude-specific QA skill.",
        );

        let snapshot = scan(workspace.path(), home.path()).unwrap();
        let qa_items = snapshot
            .artifacts
            .iter()
            .filter(|item| item.kind == HarnessKind::Skill && item.name == "qa")
            .collect::<Vec<_>>();

        assert_eq!(qa_items.len(), 2);
        assert!(qa_items.iter().any(|item| {
            item.resolution == ResolutionState::Effective && item.counterpart_id.is_some()
        }));
        assert!(qa_items
            .iter()
            .any(|item| item.resolution == ResolutionState::Defined));
        assert!(snapshot.warnings.iter().any(|warning| {
            warning.id.starts_with("counterpart-difference:Repo:")
                && warning.id.ends_with(":Skill:qa")
                && matches!(&warning.severity, &WarningSeverity::Info)
        }));
    }

    #[test]
    fn compares_counterparts_only_within_the_same_nested_project_layer() {
        let home = tempdir().unwrap();
        let repository = tempdir().unwrap();
        initialize_git_repository(repository.path());
        let package = repository.path().join("packages");
        let workspace = package.join("app");
        fs::create_dir_all(&workspace).unwrap();

        write_skill(
            &package.join(".agents/skills/qa/SKILL.md"),
            "qa",
            "Parent shared QA skill.",
        );
        write_skill(
            &workspace.join(".agents/skills/qa/SKILL.md"),
            "qa",
            "Child shared QA skill.",
        );
        write_skill(
            &workspace.join(".claude/skills/qa/SKILL.md"),
            "qa",
            "Child Claude QA skill.",
        );

        let snapshot = scan(&workspace, home.path()).unwrap();
        let parent = snapshot
            .artifacts
            .iter()
            .find(|item| item.path.ends_with("packages/.agents/skills/qa/SKILL.md"))
            .expect("parent-layer QA skill");
        let child_items = snapshot
            .artifacts
            .iter()
            .filter(|item| {
                item.name == "qa"
                    && item.scope == HarnessScope::Nested
                    && item.path.contains("packages/app/")
            })
            .collect::<Vec<_>>();
        let warnings = snapshot
            .warnings
            .iter()
            .filter(|warning| warning.id.starts_with("counterpart-difference:Nested:"))
            .collect::<Vec<_>>();

        assert_eq!(child_items.len(), 2);
        assert!(parent.counterpart_id.is_none());
        assert!(child_items.iter().all(|item| item.counterpart_id.is_some()));
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].artifact_ids.len(), 2);
        assert!(!warnings[0].artifact_ids.contains(&parent.id));
        assert!(child_items
            .iter()
            .all(|item| warnings[0].artifact_ids.contains(&item.id)));
    }

    #[test]
    fn does_not_report_counterpart_difference_across_scopes() {
        let home = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        write_skill(
            &home.path().join(".agents/skills/qa/SKILL.md"),
            "qa",
            "User QA skill.",
        );
        write_skill(
            &workspace.path().join(".claude/skills/qa/SKILL.md"),
            "qa",
            "Project QA skill.",
        );

        let snapshot = scan(workspace.path(), home.path()).unwrap();
        let qa_items = snapshot
            .artifacts
            .iter()
            .filter(|item| item.kind == HarnessKind::Skill && item.name == "qa")
            .collect::<Vec<_>>();

        assert_eq!(qa_items.len(), 2);
        assert!(qa_items.iter().any(|item| item.scope == HarnessScope::User));
        assert!(qa_items.iter().any(|item| item.scope == HarnessScope::Repo));
        assert!(qa_items.iter().all(|item| item.counterpart_id.is_none()));
        assert!(!snapshot
            .warnings
            .iter()
            .any(|warning| warning.id.starts_with("counterpart-difference:")));
    }

    #[cfg(unix)]
    #[test]
    fn gives_real_and_symlinked_memory_unique_ids_and_only_edits_the_real_source() {
        use std::os::unix::fs::symlink;

        let home = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        initialize_git_repository(workspace.path());
        fs::create_dir_all(workspace.path().join(".codex/memories")).unwrap();
        let target = workspace.path().join(".codex/memories/real.md");
        fs::write(&target, "Editable project memory.").unwrap();
        symlink("real.md", workspace.path().join(".codex/memories/alias.md")).unwrap();

        let snapshot = scan(workspace.path(), home.path()).unwrap();
        let memories = snapshot
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind == HarnessKind::Memory)
            .collect::<Vec<_>>();
        let real = memories
            .iter()
            .find(|artifact| artifact.editable)
            .expect("the real memory source remains editable");
        let alias = memories
            .iter()
            .find(|artifact| !artifact.editable)
            .expect("the symbolic-link alias remains inspectable but view-only");
        let artifact_ids = snapshot
            .artifacts
            .iter()
            .map(|artifact| artifact.id.as_str())
            .collect::<HashSet<_>>();

        assert_eq!(memories.len(), 2);
        assert_ne!(real.id, alias.id);
        assert_eq!(artifact_ids.len(), snapshot.artifacts.len());
        assert_eq!(real.path, target.canonicalize().unwrap().to_string_lossy());
        assert_eq!(alias.path, real.path);
        assert!(alias
            .editability_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("symbolic links")));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_repo_symlink_that_resolves_outside_repository() {
        use std::os::unix::fs::symlink;

        let home = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let outside = tempdir().unwrap();
        initialize_git_repository(workspace.path());
        fs::create_dir_all(workspace.path().join(".codex")).unwrap();
        let secret_path = outside.path().join("secret.toml");
        fs::write(&secret_path, "api_key = \"must-never-be-read\"").unwrap();
        symlink(&secret_path, workspace.path().join(".codex/config.toml")).unwrap();

        let snapshot = scan(workspace.path(), home.path()).unwrap();

        assert!(!snapshot
            .artifacts
            .iter()
            .any(|artifact| artifact.path == secret_path.to_string_lossy()));
        let warning = snapshot
            .warnings
            .iter()
            .find(|warning| warning.id == "scan-incomplete")
            .expect("an escaped symlink must make the snapshot explicitly incomplete");
        assert!(warning.detail.contains("outside authorized root"));
        assert!(!warning.detail.contains("must-never-be-read"));
    }

    #[test]
    fn skips_files_above_hard_cap_and_reports_incomplete_scan() {
        let home = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let agents = workspace.path().join(".codex/agents");
        fs::create_dir_all(&agents).unwrap();
        let oversized_path = agents.join("oversized.toml");
        let oversized = File::create(&oversized_path).unwrap();
        oversized.set_len(MAX_FILE_BYTES + 1).unwrap();

        let snapshot = scan(workspace.path(), home.path()).unwrap();

        assert!(!snapshot
            .artifacts
            .iter()
            .any(|artifact| artifact.path.ends_with("oversized.toml")));
        let warning = snapshot
            .warnings
            .iter()
            .find(|warning| warning.id == "scan-incomplete")
            .expect("an oversized candidate must make the snapshot incomplete");
        assert!(warning.detail.contains("per-file scan limit"));
    }
}
