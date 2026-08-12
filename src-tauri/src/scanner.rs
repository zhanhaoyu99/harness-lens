use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufReader, Read},
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use walkdir::{DirEntry, WalkDir};

use crate::{
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

    let mut warnings = annotate_duplicates_and_drift(&mut artifacts);
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
    warnings.sort_by_key(|warning| match &warning.severity {
        WarningSeverity::Error => 0,
        WarningSeverity::Warning => 1,
        WarningSeverity::Info => 2,
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
    collect_claude_project_memories(
        &claude.join("projects"),
        [workspace, repo_root],
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
            scope,
            output,
            diagnostics,
        );
    }

    let codex = repo_root.join(".codex");
    push_if_file(
        output,
        codex.join("config.toml"),
        None,
        HarnessKind::Config,
        HarnessProvider::Codex,
        HarnessScope::Repo,
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
        HarnessScope::Repo,
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
        HarnessScope::Repo,
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
        HarnessScope::Repo,
        ResolutionState::Defined,
        "Discovered in the repository Codex rules directory; runtime resolution is not observed.",
        output,
        diagnostics,
    );
    collect_skill_dir(
        &codex.join("skills"),
        HarnessProvider::Codex,
        HarnessScope::Repo,
        ResolutionState::Defined,
        "Discovered in the repository Codex skill directory; invocation is not observed.",
        output,
        diagnostics,
    );
    collect_memory_source(
        &codex.join("memories"),
        8,
        &["md", "txt", "json", "jsonl", "toml", "yaml", "yml"],
        HarnessProvider::Codex,
        HarnessScope::Repo,
        ResolutionState::Defined,
        "Repository memory metadata discovered; runtime loading is not observed.",
        output,
        diagnostics,
    );
    collect_agent_dir(
        &codex.join("agents"),
        HarnessProvider::Codex,
        HarnessScope::Repo,
        output,
        diagnostics,
    );

    let claude = repo_root.join(".claude");
    for name in ["settings.json", "settings.local.json"] {
        push_if_file(
            output,
            claude.join(name),
            None,
            HarnessKind::Config,
            HarnessProvider::Claude,
            HarnessScope::Repo,
            ResolutionState::Defined,
            "Discovered; effective status requires Claude runtime evidence.",
            true,
            false,
        );
    }
    collect_skill_dir(
        &claude.join("skills"),
        HarnessProvider::Claude,
        HarnessScope::Repo,
        ResolutionState::Defined,
        "Discovered; actual invocation requires Claude runtime evidence.",
        output,
        diagnostics,
    );
    collect_agent_dir(
        &claude.join("agents"),
        HarnessProvider::Claude,
        HarnessScope::Repo,
        output,
        diagnostics,
    );
    collect_matching_files(
        &claude.join("rules"),
        8,
        &["md", "txt", "json", "toml", "yaml", "yml"],
        HarnessKind::Rule,
        HarnessProvider::Claude,
        HarnessScope::Repo,
        ResolutionState::Defined,
        "Repository Claude rule discovered; runtime resolution is not observed.",
        output,
        diagnostics,
    );
    collect_matching_files(
        &claude.join("commands"),
        8,
        &["md"],
        HarnessKind::Skill,
        HarnessProvider::Claude,
        HarnessScope::Repo,
        ResolutionState::Defined,
        "Legacy repository Claude command discovered; invocation is not observed.",
        output,
        diagnostics,
    );
    for memory_path in [claude.join("memory"), claude.join("memories")] {
        collect_memory_source(
            &memory_path,
            8,
            &["md", "txt", "json", "jsonl", "toml", "yaml", "yml"],
            HarnessProvider::Claude,
            HarnessScope::Repo,
            ResolutionState::Defined,
            "Repository Claude memory metadata discovered; runtime loading is not observed.",
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
    let canonical_path = candidate
        .path
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let allowed_root = authorized_root(&candidate, repo_root, home)
        .canonicalize()
        .map_err(|error| format!("Unable to resolve authorized root: {error}"))?;
    if !canonical_path.starts_with(&allowed_root) {
        return Err(format!(
            "Resolved outside authorized root {} and was skipped.",
            allowed_root.to_string_lossy()
        ));
    }

    let metadata = fs::metadata(&canonical_path).map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("Resolved path is not a regular file.".to_string());
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(format!(
            "File exceeds the {} MiB per-file scan limit.",
            MAX_FILE_BYTES / (1024 * 1024)
        ));
    }
    if metadata.len() > remaining_total_bytes {
        return Err(format!(
            "Total scan byte budget reached ({} MiB).",
            MAX_TOTAL_FILE_BYTES / (1024 * 1024)
        ));
    }

    let hard_read_limit = MAX_FILE_BYTES.min(remaining_total_bytes);
    let (raw_content, truncated, content_hash) =
        read_preview_and_hash(&canonical_path, MAX_CONTENT_BYTES, hard_read_limit)?;
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
    let id_seed = format!(
        "{:?}:{:?}:{}",
        candidate.provider, candidate.kind, path_string
    );
    let id = hex::encode(Sha256::digest(id_seed.as_bytes()))[..24].to_string();

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
        modified_at: metadata.modified().ok().map(system_time_to_rfc3339),
        size_bytes: metadata.len(),
        resolution: candidate.resolution,
        resolution_reason: candidate.reason,
        duplicate_group_id: None,
        counterpart_id: None,
        description,
        sensitive: candidate.sensitive,
        truncated,
    })
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

fn read_preview_and_hash(
    path: &Path,
    preview_limit: u64,
    hard_read_limit: u64,
) -> Result<(String, bool, String), String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut reader = BufReader::new(file);
    let preview_limit = usize::try_from(preview_limit).unwrap_or(usize::MAX);
    let mut preview = Vec::with_capacity(preview_limit.min(16 * 1024));
    let mut buffer = [0_u8; 16 * 1024];
    let mut total_bytes = 0_u64;
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
        total_bytes = total_bytes.saturating_add(count as u64);
        if preview.len() < preview_limit {
            let remaining = preview_limit - preview.len();
            preview.extend_from_slice(&buffer[..count.min(remaining)]);
        }
    }

    Ok((
        String::from_utf8_lossy(&preview).into_owned(),
        total_bytes > preview_limit as u64,
        hex::encode(hasher.finalize()),
    ))
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
    if !content.starts_with("---") {
        return (None, None);
    }
    let block = content
        .strip_prefix("---")
        .and_then(|remaining| remaining.split_once("---").map(|(head, _)| head));
    let Some(block) = block else {
        return (None, None);
    };
    let mut name = None;
    let mut description = None;
    for line in block.lines() {
        if let Some(value) = line.strip_prefix("name:") {
            name = Some(value.trim().trim_matches(['\"', '\'']).to_string());
        }
        if let Some(value) = line.strip_prefix("description:") {
            description = Some(value.trim().trim_matches(['\"', '\'']).to_string());
        }
    }
    (name, description)
}

fn annotate_duplicates_and_drift(artifacts: &mut [HarnessArtifact]) -> Vec<HarnessWarning> {
    let mut warnings = Vec::new();
    let mut by_hash: HashMap<(String, String), Vec<usize>> = HashMap::new();
    let mut by_name: HashMap<(String, String), Vec<usize>> = HashMap::new();
    for (index, artifact) in artifacts.iter().enumerate() {
        let kind = format!("{:?}", artifact.kind);
        by_hash
            .entry((kind.clone(), artifact.content_hash.clone()))
            .or_default()
            .push(index);
        by_name
            .entry((kind, artifact.name.to_lowercase()))
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

    for ((kind, name), indexes) in by_name.into_iter().filter(|(_, indexes)| indexes.len() > 1) {
        let hashes = indexes
            .iter()
            .map(|index| artifacts[*index].content_hash.clone())
            .collect::<std::collections::HashSet<_>>();
        let providers = indexes
            .iter()
            .map(|index| format!("{:?}", artifacts[*index].provider))
            .collect::<std::collections::HashSet<_>>();
        if hashes.len() <= 1 || providers.len() <= 1 {
            continue;
        }
        let ids = indexes
            .iter()
            .map(|index| artifacts[*index].id.clone())
            .collect::<Vec<_>>();
        let peers = indexes
            .iter()
            .map(|index| {
                (
                    artifacts[*index].id.clone(),
                    artifacts[*index].provider.clone(),
                )
            })
            .collect::<Vec<_>>();
        for index in indexes {
            let provider = artifacts[index].provider.clone();
            artifacts[index].counterpart_id = peers
                .iter()
                .find(|(_, candidate_provider)| *candidate_provider != provider)
                .map(|(id, _)| id.clone());
        }
        warnings.push(HarnessWarning {
            id: format!("drift:{kind}:{name}"),
            severity: WarningSeverity::Warning,
            title: format!("Provider drift: {name}"),
            detail: "Same-name Harness items differ across providers.".to_string(),
            artifact_ids: ids,
        });
    }

    warnings
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
    use std::{fs, fs::File, path::Path, process::Command};

    use tempfile::tempdir;

    use super::{scan, MAX_CONTENT_BYTES, MAX_FILE_BYTES};
    use crate::model::{HarnessKind, HarnessScope, ResolutionState};

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
    fn reports_cross_provider_drift_without_overwriting_resolution() {
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
        assert!(snapshot
            .warnings
            .iter()
            .any(|warning| warning.id == "drift:Skill:qa"));
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
