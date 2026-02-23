//! File security module.
//!
//! Implements security-related file utilities:
//! - shred: Overwrite file contents to impede recovery, then optionally remove
//! - truncate_file: Set a file to a specific size
//! - mktemp: Create a temporary file or directory inside the sandbox

use crate::tools::sandbox::Sandbox;
use crate::types::{AgentError, AgentResult};
use std::path::Path;

/// Shred - overwrite a file's contents for `passes` iterations to make
/// recovery difficult, then optionally remove it.
///
/// Each pass writes a different byte pattern (`0x00`, `0xFF`, `0xAA`, cycling).
/// The file must reside within the sandbox.
///
/// # Examples
///
/// ```no_run
/// use mistralrs_agent_tools::tools::sandbox::Sandbox;
/// use mistralrs_agent_tools::tools::security::shred;
/// use mistralrs_agent_tools::types::SandboxConfig;
/// use std::path::Path;
///
/// // let sandbox = Sandbox::new(SandboxConfig::new(root));
/// // shred(&sandbox, Path::new("secret.txt"), 3, true).unwrap();
/// ```
pub fn shred(sandbox: &Sandbox, path: &Path, passes: usize, remove: bool) -> AgentResult<String> {
    // validate_write confirms the path is within the sandbox
    let resolved = sandbox.validate_write(path)?;

    if !resolved.exists() {
        return Err(AgentError::NotFound(format!(
            "File not found: {}",
            path.display()
        )));
    }
    if !resolved.is_file() {
        return Err(AgentError::InvalidInput(format!(
            "Not a regular file: {}",
            path.display()
        )));
    }

    let file_size = std::fs::metadata(&resolved)
        .map_err(|e| AgentError::IoError(e.to_string()))?
        .len() as usize;

    // Overwrite with alternating byte patterns
    for pass in 0..passes {
        let pattern: u8 = match pass % 3 {
            0 => 0x00,
            1 => 0xFF,
            _ => 0xAA,
        };
        let data = vec![pattern; file_size];
        std::fs::write(&resolved, &data).map_err(|e| AgentError::IoError(e.to_string()))?;
    }

    if remove {
        std::fs::remove_file(&resolved).map_err(|e| AgentError::IoError(e.to_string()))?;
        Ok(format!(
            "shred: {}: removed after {} passes",
            path.display(),
            passes
        ))
    } else {
        Ok(format!(
            "shred: {}: overwritten {} passes",
            path.display(),
            passes
        ))
    }
}

/// Truncate - set a file to exactly `size` bytes, creating it if necessary.
///
/// Extending a file fills the new region with null bytes (platform-dependent
/// behaviour; on most systems this creates a sparse file).
///
/// # Examples
///
/// ```no_run
/// use mistralrs_agent_tools::tools::sandbox::Sandbox;
/// use mistralrs_agent_tools::tools::security::truncate_file;
/// use mistralrs_agent_tools::types::SandboxConfig;
/// use std::path::Path;
///
/// // truncate_file(&sandbox, Path::new("file.txt"), 1024).unwrap();
/// ```
pub fn truncate_file(sandbox: &Sandbox, path: &Path, size: u64) -> AgentResult<String> {
    let resolved = sandbox.validate_write(path)?;

    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&resolved)
        .map_err(|e| AgentError::IoError(e.to_string()))?;

    file.set_len(size)
        .map_err(|e| AgentError::IoError(e.to_string()))?;

    Ok(format!(
        "truncate: {} set to {} bytes",
        path.display(),
        size
    ))
}

/// Mktemp - create a uniquely-named temporary file or directory inside the
/// sandbox root and return its absolute path.
///
/// The name is formed from `prefix` (default `"tmp"`) followed by a dot and a
/// nanosecond-resolution timestamp.  The sandbox's `validate_write` check is
/// applied before creation.
///
/// # Examples
///
/// ```no_run
/// use mistralrs_agent_tools::tools::sandbox::Sandbox;
/// use mistralrs_agent_tools::tools::security::mktemp;
/// use mistralrs_agent_tools::types::SandboxConfig;
///
/// // let path = mktemp(&sandbox, false, Some("scratch")).unwrap();
/// // assert!(std::path::Path::new(&path).is_file());
/// ```
pub fn mktemp(sandbox: &Sandbox, directory: bool, prefix: Option<&str>) -> AgentResult<String> {
    let pfx = prefix.unwrap_or("tmp");

    // Generate a name that is highly unlikely to collide
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let name = format!("{}.{}", pfx, timestamp);
    let target = sandbox.root().join(&name);

    // Confirm the target is within the sandbox before creating anything
    sandbox.validate_write(&target)?;

    if directory {
        std::fs::create_dir_all(&target).map_err(|e| AgentError::IoError(e.to_string()))?;
    } else {
        // Ensure the parent directory exists (it should be the sandbox root)
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AgentError::IoError(e.to_string()))?;
        }
        std::fs::File::create(&target).map_err(|e| AgentError::IoError(e.to_string()))?;
    }

    Ok(target.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SandboxConfig;
    use tempfile::TempDir;

    fn test_sandbox() -> (TempDir, Sandbox) {
        let dir = TempDir::new().unwrap();
        let sandbox = Sandbox::new(SandboxConfig::new(dir.path().to_path_buf()));
        (dir, sandbox)
    }

    #[test]
    fn test_shred_overwrites_content() {
        let (dir, sandbox) = test_sandbox();
        let file = dir.path().join("secret.txt");
        std::fs::write(&file, "secret data").unwrap();

        let result = shred(&sandbox, &file, 3, false).unwrap();
        assert!(result.contains("overwritten"));

        // File still exists but content has changed
        let content = std::fs::read(&file).unwrap();
        assert_ne!(content, b"secret data");
    }

    #[test]
    fn test_shred_remove() {
        let (dir, sandbox) = test_sandbox();
        let file = dir.path().join("to_delete.txt");
        std::fs::write(&file, "delete me").unwrap();

        let result = shred(&sandbox, &file, 1, true).unwrap();
        assert!(result.contains("removed"));
        assert!(!file.exists());
    }

    #[test]
    fn test_shred_nonexistent_file() {
        let (_dir, sandbox) = test_sandbox();
        // validate_write will succeed for a path in the sandbox root, but the
        // subsequent existence check should fail.
        let fake = sandbox.root().join("nonexistent.bin");
        assert!(shred(&sandbox, &fake, 1, false).is_err());
    }

    #[test]
    fn test_shred_sandbox_violation() {
        let (_dir, sandbox) = test_sandbox();
        #[cfg(windows)]
        let outside = std::path::PathBuf::from("C:\\Windows\\System32\\notepad.exe");
        #[cfg(not(windows))]
        let outside = std::path::PathBuf::from("/etc/passwd");

        let result = shred(&sandbox, &outside, 1, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_truncate_shrink() {
        let (dir, sandbox) = test_sandbox();
        let file = dir.path().join("truncate_me.txt");
        std::fs::write(&file, "hello world").unwrap();

        let result = truncate_file(&sandbox, &file, 5).unwrap();
        assert!(result.contains("5 bytes"));

        let content = std::fs::read(&file).unwrap();
        assert_eq!(content.len(), 5);
        assert_eq!(&content, b"hello");
    }

    #[test]
    fn test_truncate_extend() {
        let (dir, sandbox) = test_sandbox();
        let file = dir.path().join("extend_me.txt");
        std::fs::write(&file, "hi").unwrap();

        truncate_file(&sandbox, &file, 10).unwrap();

        let content = std::fs::read(&file).unwrap();
        assert_eq!(content.len(), 10);
    }

    #[test]
    fn test_truncate_create_new() {
        let (dir, sandbox) = test_sandbox();
        let file = dir.path().join("new_file.bin");

        truncate_file(&sandbox, &file, 64).unwrap();

        assert!(file.exists());
        let content = std::fs::read(&file).unwrap();
        assert_eq!(content.len(), 64);
    }

    #[test]
    fn test_mktemp_file() {
        let (_dir, sandbox) = test_sandbox();

        let result = mktemp(&sandbox, false, Some("test")).unwrap();
        let path = std::path::Path::new(&result);
        assert!(path.exists(), "mktemp file should exist");
        assert!(path.is_file(), "mktemp result should be a regular file");
    }

    #[test]
    fn test_mktemp_directory() {
        let (_dir, sandbox) = test_sandbox();

        let result = mktemp(&sandbox, true, Some("testdir")).unwrap();
        let path = std::path::Path::new(&result);
        assert!(path.exists(), "mktemp dir should exist");
        assert!(path.is_dir(), "mktemp result should be a directory");
    }

    #[test]
    fn test_mktemp_default_prefix() {
        let (_dir, sandbox) = test_sandbox();

        let result = mktemp(&sandbox, false, None).unwrap();
        let filename = std::path::Path::new(&result)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(
            filename.starts_with("tmp."),
            "default prefix should be 'tmp'"
        );
    }

    #[test]
    fn test_mktemp_within_sandbox() {
        let (_dir, sandbox) = test_sandbox();

        let result = mktemp(&sandbox, false, Some("bounded")).unwrap();
        let path = std::path::Path::new(&result);
        assert!(
            path.starts_with(sandbox.root()),
            "mktemp result must be within sandbox"
        );
    }
}
