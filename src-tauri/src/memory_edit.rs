use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Component, Path},
};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;

use crate::FileIdentity;

pub(crate) const MAX_MEMORY_BYTES: u64 = 256 * 1024;

#[derive(Debug)]
pub(crate) struct LoadedMemory {
    pub content: String,
    pub identity: FileIdentity,
    pub safe_to_edit: bool,
    pub unsafe_reason: Option<String>,
}

pub(crate) fn memory_editability(path: &Path) -> (bool, Option<String>) {
    let is_markdown = path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("md"));
    if !is_markdown {
        return (
            false,
            Some("Only Markdown memory files can be edited.".to_string()),
        );
    }

    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(
        file_name.as_str(),
        "memory_summary.md" | "raw_memories.md" | "raw_memories"
    ) {
        return (
            false,
            Some("Generated memory indexes are view-only.".to_string()),
        );
    }

    let components = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_ascii_lowercase()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let provider_index = components
        .iter()
        .rposition(|component| component == ".codex" || component == ".claude");
    let memory_root_index = provider_index.and_then(|provider_index| {
        components
            .iter()
            .enumerate()
            .skip(provider_index + 1)
            .filter(|(_, component)| {
                component.as_str() == "memory" || component.as_str() == "memories"
            })
            .map(|(index, _)| index + 1)
            .next_back()
    });
    let directory_end = components.len().saturating_sub(1);
    let has_view_only_directory = memory_root_index.is_some_and(|start| {
        components[start..directory_end].iter().any(|component| {
            matches!(
                component.as_str(),
                "raw_memories" | "rollout_summaries" | "skills"
            )
        })
    });
    if has_view_only_directory {
        return (
            false,
            Some("Runtime evidence and skill-owned memory files are view-only.".to_string()),
        );
    }

    (true, None)
}

pub(crate) fn load_memory_file(
    path: &Path,
    expected_identity: &FileIdentity,
) -> Result<LoadedMemory, String> {
    let (bytes, identity, metadata) = read_file(path, MAX_MEMORY_BYTES)?;
    if &identity != expected_identity {
        return Err(
            "The memory file changed after the scan. Rescan before viewing it.".to_string(),
        );
    }
    if bytes.contains(&0) {
        return Err("Memory files containing NUL bytes are not supported.".to_string());
    }
    let content = String::from_utf8(bytes)
        .map_err(|_| "Memory files must contain valid UTF-8 text.".to_string())?;
    let (mut safe_to_edit, mut unsafe_reason) = edit_path_safety(path, &metadata);
    if safe_to_edit && content.starts_with('\u{feff}') {
        safe_to_edit = false;
        unsafe_reason = Some("Memory files with a byte-order mark are view-only.".to_string());
    } else if safe_to_edit && content.contains('\r') {
        safe_to_edit = false;
        unsafe_reason = Some(
            "Memory files with CR or CRLF line endings are view-only to preserve exact bytes."
                .to_string(),
        );
    }

    Ok(LoadedMemory {
        content,
        identity,
        safe_to_edit,
        unsafe_reason,
    })
}

pub(crate) fn validate_memory_content(content: &str) -> Result<(), String> {
    if content.as_bytes().contains(&0) {
        return Err("Memory files containing NUL bytes are not supported.".to_string());
    }
    if content.starts_with('\u{feff}') {
        return Err(
            "Memory content with a byte-order mark is not supported for editing.".to_string(),
        );
    }
    if content.contains('\r') {
        return Err(
            "Memory content must use LF line endings; CR and CRLF are not supported for editing."
                .to_string(),
        );
    }
    if content.len() as u64 > MAX_MEMORY_BYTES {
        return Err(format!(
            "Memory content exceeds the {} KiB edit limit.",
            MAX_MEMORY_BYTES / 1024
        ));
    }
    Ok(())
}

pub(crate) fn verify_memory_revision(
    path: &Path,
    expected_identity: &FileIdentity,
) -> Result<(), String> {
    let (_, current_identity, metadata) = read_file(path, MAX_MEMORY_BYTES)?;
    let (safe_to_edit, reason) = edit_path_safety(path, &metadata);
    if !safe_to_edit {
        return Err(reason.unwrap_or_else(|| "The memory file is not safe to edit.".to_string()));
    }
    if &current_identity != expected_identity {
        return Err(
            "The memory file changed after it was loaded. Reload before saving.".to_string(),
        );
    }
    Ok(())
}

pub(crate) fn replace_memory_file(
    path: &Path,
    expected_identity: &FileIdentity,
    content: &str,
) -> Result<FileIdentity, String> {
    replace_memory_file_with_commit(
        path,
        expected_identity,
        content,
        |temporary, destination| {
            temporary
                .persist(destination)
                .map(|_| ())
                .map_err(|error| format!("Unable to replace the memory file: {}", error.error))
        },
    )
}

fn replace_memory_file_with_commit<F>(
    path: &Path,
    expected_identity: &FileIdentity,
    content: &str,
    commit: F,
) -> Result<FileIdentity, String>
where
    F: FnOnce(NamedTempFile, &Path) -> Result<(), String>,
{
    validate_memory_content(content)?;
    verify_memory_revision(path, expected_identity)?;

    let parent = path
        .parent()
        .ok_or_else(|| "Memory file has no parent directory.".to_string())?;
    let expected_parent = parent
        .canonicalize()
        .map_err(|error| format!("Unable to verify the memory directory: {error}"))?;
    let canonical_file = path
        .canonicalize()
        .map_err(|error| format!("Unable to verify the memory file: {error}"))?;
    if canonical_file.parent() != Some(expected_parent.as_path()) {
        return Err("The memory directory resolves through a symbolic link.".to_string());
    }

    let existing_metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Unable to inspect the memory file: {error}"))?;
    let existing_permissions = existing_metadata.permissions();
    let mut temporary = NamedTempFile::new_in(parent)
        .map_err(|error| format!("Unable to create a temporary memory file: {error}"))?;
    temporary
        .write_all(content.as_bytes())
        .map_err(|error| format!("Unable to write the temporary memory file: {error}"))?;
    temporary
        .flush()
        .map_err(|error| format!("Unable to flush the temporary memory file: {error}"))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| format!("Unable to sync the temporary memory file: {error}"))?;
    fs::set_permissions(temporary.path(), existing_permissions)
        .map_err(|error| format!("Unable to preserve memory file permissions: {error}"))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| format!("Unable to sync memory file permissions: {error}"))?;

    let prepared_metadata = temporary
        .as_file()
        .metadata()
        .map_err(|error| format!("Unable to verify the prepared memory file: {error}"))?;
    if prepared_metadata.len() != content.len() as u64 {
        return Err("Prepared memory content has an unexpected length.".to_string());
    }
    let saved_identity = identity_from_bytes(&prepared_metadata, content.as_bytes());

    // Recheck immediately before the atomic replace so a stale editor cannot overwrite
    // a newer version while the temporary file is being prepared.
    verify_memory_revision(path, expected_identity)?;

    // The commit is deliberately the final fallible operation. Once the atomic replace
    // succeeds, the caller must receive a committed result rather than a later reopen/fsync
    // error that could incorrectly claim the on-disk write failed.
    commit(temporary, path)?;
    Ok(saved_identity)
}

fn read_file(path: &Path, max_bytes: u64) -> Result<(Vec<u8>, FileIdentity, fs::Metadata), String> {
    let path_metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Unable to inspect the memory file: {error}"))?;
    if path_metadata.file_type().is_symlink() {
        return Err("Symbolic-link memory files are not supported.".to_string());
    }
    if !path_metadata.is_file() {
        return Err("Resolved memory path is not a regular file.".to_string());
    }

    let mut file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|error| format!("Unable to read the memory file: {error}"))?;
    let before = file
        .metadata()
        .map_err(|error| format!("Unable to inspect the open memory file: {error}"))?;
    if before.len() > max_bytes {
        return Err(format!(
            "Memory file exceeds the {} KiB view limit.",
            max_bytes / 1024
        ));
    }

    let mut bytes = Vec::with_capacity(before.len() as usize);
    (&mut file)
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Unable to read the memory file: {error}"))?;
    if bytes.len() as u64 > max_bytes {
        return Err(format!(
            "Memory file exceeds the {} KiB view limit.",
            max_bytes / 1024
        ));
    }

    let after = file
        .metadata()
        .map_err(|error| format!("Unable to recheck the open memory file: {error}"))?;
    if !same_open_file_revision(&before, &after) || after.len() != bytes.len() as u64 {
        return Err("Memory file changed while it was being read.".to_string());
    }

    let identity = identity_from_bytes(&after, &bytes);
    Ok((bytes, identity, after))
}

fn identity_from_bytes(metadata: &fs::Metadata, bytes: &[u8]) -> FileIdentity {
    FileIdentity {
        len: metadata.len(),
        modified: metadata.modified().ok(),
        content_hash: hex::encode(Sha256::digest(bytes)),
        #[cfg(unix)]
        device: metadata.dev(),
        #[cfg(unix)]
        inode: metadata.ino(),
    }
}

fn same_open_file_revision(before: &fs::Metadata, after: &fs::Metadata) -> bool {
    before.len() == after.len() && before.modified().ok() == after.modified().ok() && {
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

fn edit_safety(metadata: &fs::Metadata) -> (bool, Option<String>) {
    #[cfg(unix)]
    {
        if metadata.uid() != rustix::process::geteuid().as_raw() {
            return (
                false,
                Some("Memory files owned by another user are view-only.".to_string()),
            );
        }
        if metadata.nlink() > 1 {
            return (
                false,
                Some("Memory files with multiple hard links are view-only.".to_string()),
            );
        }
        if metadata.permissions().mode() & 0o7000 != 0 {
            return (
                false,
                Some("Memory files with special permission bits are view-only.".to_string()),
            );
        }
        if metadata.permissions().mode() & 0o200 == 0 {
            return (
                false,
                Some("Memory files without owner-write permission are view-only.".to_string()),
            );
        }
    }
    (true, None)
}

fn edit_path_safety(path: &Path, metadata: &fs::Metadata) -> (bool, Option<String>) {
    let file_safety = edit_safety(metadata);
    if !file_safety.0 {
        return file_safety;
    }

    #[cfg(unix)]
    {
        let Some(parent) = path.parent() else {
            return (
                false,
                Some("Memory file has no parent directory.".to_string()),
            );
        };
        let parent_metadata = match fs::symlink_metadata(parent) {
            Ok(metadata) => metadata,
            Err(_) => {
                return (
                    false,
                    Some("The Memory directory could not be verified.".to_string()),
                )
            }
        };
        let mode = parent_metadata.permissions().mode();
        if !parent_metadata.is_dir()
            || parent_metadata.uid() != rustix::process::geteuid().as_raw()
            || mode & 0o300 != 0o300
            || mode & 0o022 != 0
        {
            return (
                false,
                Some(
                    "Memory files in directories with unsafe ownership or permissions are view-only."
                        .to_string(),
                ),
            );
        }
    }

    (true, None)
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[cfg(unix)]
    use std::os::unix::fs::{symlink, PermissionsExt};

    use super::*;
    use crate::file_identity;

    #[test]
    fn generated_and_non_markdown_memories_are_view_only() {
        assert!(!memory_editability(Path::new("memory_summary.md")).0);
        assert!(!memory_editability(Path::new("raw_memories.md")).0);
        assert!(
            !memory_editability(Path::new(
                "/project/.codex/memories/rollout_summaries/run.md"
            ))
            .0
        );
        assert!(
            !memory_editability(Path::new(
                "/project/.codex/memories/skills/example/SKILL.md"
            ))
            .0
        );
        assert!(!memory_editability(Path::new("memory.json")).0);
        assert!(memory_editability(Path::new("MEMORY.md")).0);
        assert!(
            memory_editability(Path::new(
                "/project/skills/mobile/.codex/memories/MEMORY.md"
            ))
            .0
        );
    }

    #[test]
    fn load_rejects_invalid_utf8_nul_and_oversized_content() {
        let directory = tempfile::tempdir().expect("temporary directory");

        let invalid_utf8 = directory.path().join("invalid.md");
        fs::write(&invalid_utf8, [0xff]).expect("invalid UTF-8 fixture");
        let identity = file_identity(&invalid_utf8, None).expect("fixture identity");
        assert!(load_memory_file(&invalid_utf8, &identity)
            .expect_err("invalid UTF-8 should fail")
            .contains("UTF-8"));

        let nul = directory.path().join("nul.md");
        fs::write(&nul, b"one\0two").expect("NUL fixture");
        let identity = file_identity(&nul, None).expect("fixture identity");
        assert!(load_memory_file(&nul, &identity)
            .expect_err("NUL should fail")
            .contains("NUL"));

        assert!(
            validate_memory_content(&"x".repeat(MAX_MEMORY_BYTES as usize + 1))
                .expect_err("oversized content should fail")
                .contains("256 KiB")
        );
        assert!(validate_memory_content("\u{feff}# Memory\n")
            .expect_err("BOM draft should fail")
            .contains("byte-order mark"));
        assert!(validate_memory_content("# Memory\r\n")
            .expect_err("CRLF draft should fail")
            .contains("LF line endings"));
        assert!(validate_memory_content("# Memory\r")
            .expect_err("CR draft should fail")
            .contains("LF line endings"));
    }

    #[test]
    fn byte_order_marks_and_crlf_are_view_only() {
        let directory = tempfile::tempdir().expect("temporary directory");
        for (name, content) in [
            ("bom.md", "\u{feff}# Memory\n"),
            ("crlf.md", "# Memory\r\n\r\n- item\r\n"),
        ] {
            let path = directory.path().join(name);
            fs::write(&path, content).expect("text fixture");
            let identity = file_identity(&path, None).expect("fixture identity");

            let loaded = load_memory_file(&path, &identity).expect("view-only load");

            assert!(!loaded.safe_to_edit);
            assert!(loaded.unsafe_reason.is_some());
        }
    }

    #[test]
    fn load_and_save_detect_external_changes() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("MEMORY.md");
        fs::write(&path, "before").expect("initial fixture");
        let identity = file_identity(&path, None).expect("initial identity");
        let loaded = load_memory_file(&path, &identity).expect("load memory");
        assert_eq!(loaded.content, "before");

        fs::write(&path, "external").expect("external change");
        assert!(replace_memory_file(&path, &loaded.identity, "editor")
            .expect_err("stale save should fail")
            .contains("changed"));
        assert_eq!(
            fs::read_to_string(path).expect("current content"),
            "external"
        );
    }

    #[cfg(unix)]
    #[test]
    fn owner_read_only_memory_is_view_only() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("MEMORY.md");
        fs::write(&path, "read only").expect("fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444))
            .expect("read-only fixture permissions");
        let identity = file_identity(&path, None).expect("fixture identity");

        let loaded = load_memory_file(&path, &identity).expect("view-only load");

        assert!(!loaded.safe_to_edit);
        assert!(loaded
            .unsafe_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("owner-write")));
    }

    #[cfg(unix)]
    #[test]
    fn memory_in_group_or_world_writable_directory_is_view_only() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let parent = directory.path().join("memory");
        fs::create_dir(&parent).expect("memory directory");
        let path = parent.join("MEMORY.md");
        fs::write(&path, "unsafe parent").expect("fixture");
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o777))
            .expect("unsafe parent permissions");
        let identity = file_identity(&path, None).expect("fixture identity");

        let loaded = load_memory_file(&path, &identity).expect("view-only load");

        assert!(!loaded.safe_to_edit);
        assert!(loaded
            .unsafe_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("unsafe ownership or permissions")));
    }

    #[test]
    fn atomic_save_replaces_content_and_preserves_permissions() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("MEMORY.md");
        fs::write(&path, "before").expect("initial fixture");
        #[cfg(unix)]
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("fixture permissions");
        let identity = file_identity(&path, None).expect("initial identity");

        let saved = replace_memory_file(&path, &identity, "after").expect("atomic save");

        assert_eq!(fs::read_to_string(&path).expect("saved content"), "after");
        assert_eq!(saved.content_hash, hex::encode(Sha256::digest(b"after")));
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(path)
                .expect("saved metadata")
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
    }

    #[cfg(unix)]
    #[test]
    fn successful_commit_is_not_reclassified_by_a_post_commit_reopen() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("MEMORY.md");
        fs::write(&path, "before").expect("initial fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("fixture permissions");
        let identity = file_identity(&path, None).expect("initial identity");

        let saved =
            replace_memory_file_with_commit(&path, &identity, "after", |temporary, destination| {
                temporary
                    .persist(destination)
                    .map_err(|error| error.error.to_string())?;
                // Make a hypothetical post-commit reopen fail. The save result must still
                // describe the already-committed bytes instead of reporting a false failure.
                fs::set_permissions(destination, fs::Permissions::from_mode(0o000))
                    .map_err(|error| error.to_string())
            })
            .expect("committed save");

        assert_eq!(saved.content_hash, hex::encode(Sha256::digest(b"after")));
        assert_eq!(saved.len, 5);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .expect("restore fixture permissions");
        assert_eq!(
            fs::read_to_string(path).expect("committed content"),
            "after"
        );
    }

    #[test]
    fn failed_commit_leaves_the_original_file_untouched() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("MEMORY.md");
        fs::write(&path, "before").expect("initial fixture");
        let identity = file_identity(&path, None).expect("initial identity");

        let error = replace_memory_file_with_commit(
            &path,
            &identity,
            "after",
            |_temporary, _destination| Err("injected commit failure".to_string()),
        )
        .expect_err("commit should fail");

        assert_eq!(error, "injected commit failure");
        assert_eq!(
            fs::read_to_string(path).expect("original content"),
            "before"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_links_and_multiple_hard_links_are_not_editable() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("source.md");
        let symbolic = directory.path().join("symbolic.md");
        let hard_link = directory.path().join("hard-link.md");
        fs::write(&source, "memory").expect("source fixture");
        symlink(&source, &symbolic).expect("symbolic link fixture");
        fs::hard_link(&source, &hard_link).expect("hard link fixture");

        let source_identity = file_identity(&source, None).expect("source identity");
        assert!(load_memory_file(&symbolic, &source_identity)
            .expect_err("symbolic link should fail")
            .contains("Symbolic-link"));
        let loaded = load_memory_file(&source, &source_identity).expect("hard-linked view");
        assert!(!loaded.safe_to_edit);
        assert!(replace_memory_file(&source, &source_identity, "after")
            .expect_err("hard-linked save should fail")
            .contains("hard links"));
    }

    #[cfg(unix)]
    #[test]
    fn special_permission_bits_make_memory_view_only() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("MEMORY.md");
        fs::write(&path, "memory").expect("fixture");
        // The sticky bit is a special permission bit and remains observable in
        // restricted macOS test environments that intentionally strip setuid/setgid.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o1644))
            .expect("special permissions fixture");
        let identity = file_identity(&path, None).expect("fixture identity");

        let loaded = load_memory_file(&path, &identity).expect("view-only load");

        assert!(!loaded.safe_to_edit);
        assert!(loaded
            .unsafe_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("special permission")));
    }
}
