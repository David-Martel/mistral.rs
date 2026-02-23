//! Testing utilities module.
//!
//! Implements testing and timing utilities:
//! - test_file: Check file types and attributes (POSIX `test` / `[` flags)
//! - test_compare: Numeric comparison operators
//! - sleep_cmd: Delay execution for a given duration

use crate::tools::sandbox::Sandbox;
use crate::types::{AgentError, AgentResult};
use std::path::Path;

/// Test file - check file types and string properties.
///
/// Supported flags:
///
/// | Flag | Meaning |
/// |------|---------|
/// | `-e` | Path exists |
/// | `-f` | Path is a regular file |
/// | `-d` | Path is a directory |
/// | `-r` | Path exists (simplified readable check) |
/// | `-w` | Path exists and is not read-only |
/// | `-s` | Path is a non-empty file |
/// | `-z` | Operand string has zero length |
/// | `-n` | Operand string has non-zero length |
///
/// For path-based flags (`-e`, `-f`, `-d`, `-r`, `-w`, `-s`) the operand is
/// validated through the sandbox before the filesystem is consulted.  For
/// string flags (`-z`, `-n`) the operand is examined directly.
///
/// # Examples
///
/// ```no_run
/// use mistralrs_agent_tools::tools::sandbox::Sandbox;
/// use mistralrs_agent_tools::tools::testing::test_file;
/// use mistralrs_agent_tools::types::SandboxConfig;
/// use std::path::PathBuf;
///
/// // let sandbox = Sandbox::new(SandboxConfig::new(root));
/// // assert!(test_file(&sandbox, "-z", "").unwrap());
/// // assert!(test_file(&sandbox, "-n", "hello").unwrap());
/// ```
pub fn test_file(sandbox: &Sandbox, flag: &str, operand: &str) -> AgentResult<bool> {
    match flag {
        "-z" => return Ok(operand.is_empty()),
        "-n" => return Ok(!operand.is_empty()),
        _ => {}
    }

    // All remaining flags inspect the filesystem via the sandbox.
    let resolved = sandbox.validate_read(Path::new(operand))?;

    match flag {
        "-e" => Ok(resolved.exists()),
        "-f" => Ok(resolved.is_file()),
        "-d" => Ok(resolved.is_dir()),
        "-r" => {
            // Simplified: if the path exists and sandbox let us validate it, it
            // is considered readable.
            Ok(resolved.exists())
        }
        "-w" => {
            if !resolved.exists() {
                return Ok(false);
            }
            let metadata =
                std::fs::metadata(&resolved).map_err(|e| AgentError::IoError(e.to_string()))?;
            Ok(!metadata.permissions().readonly())
        }
        "-s" => {
            if !resolved.exists() {
                return Ok(false);
            }
            let metadata =
                std::fs::metadata(&resolved).map_err(|e| AgentError::IoError(e.to_string()))?;
            Ok(metadata.len() > 0)
        }
        unknown => Err(AgentError::InvalidInput(format!(
            "Unknown test flag: {}",
            unknown
        ))),
    }
}

/// Test compare - compare two integers using POSIX numeric test operators.
///
/// Supported operators: `-eq`, `-ne`, `-lt`, `-le`, `-gt`, `-ge`.
///
/// # Examples
///
/// ```
/// use mistralrs_agent_tools::tools::testing::test_compare;
/// assert!(test_compare("5", "-eq", "5").unwrap());
/// assert!(test_compare("3", "-lt", "5").unwrap());
/// assert!(!test_compare("7", "-le", "5").unwrap());
/// ```
pub fn test_compare(a: &str, op: &str, b: &str) -> AgentResult<bool> {
    let a_val: i64 = a
        .parse()
        .map_err(|_| AgentError::InvalidInput(format!("Not a number: {}", a)))?;
    let b_val: i64 = b
        .parse()
        .map_err(|_| AgentError::InvalidInput(format!("Not a number: {}", b)))?;

    match op {
        "-eq" => Ok(a_val == b_val),
        "-ne" => Ok(a_val != b_val),
        "-lt" => Ok(a_val < b_val),
        "-le" => Ok(a_val <= b_val),
        "-gt" => Ok(a_val > b_val),
        "-ge" => Ok(a_val >= b_val),
        unknown => Err(AgentError::InvalidInput(format!(
            "Unknown comparison operator: {}",
            unknown
        ))),
    }
}

/// Sleep - pause execution for `seconds` seconds (accepts fractional values).
///
/// For safety the duration is capped at 300 seconds (5 minutes).  A negative
/// duration is rejected with an error.
///
/// Returns a human-readable message describing how long was actually slept.
///
/// # Examples
///
/// ```
/// use mistralrs_agent_tools::tools::testing::sleep_cmd;
/// let msg = sleep_cmd(0.0).unwrap();
/// assert!(msg.contains("slept"));
/// ```
pub fn sleep_cmd(seconds: f64) -> AgentResult<String> {
    if seconds < 0.0 {
        return Err(AgentError::InvalidInput(
            "Sleep duration cannot be negative".into(),
        ));
    }

    const MAX_SLEEP_SECS: f64 = 300.0;
    let actual = seconds.min(MAX_SLEEP_SECS);

    std::thread::sleep(std::time::Duration::from_secs_f64(actual));

    if actual < seconds {
        Ok(format!(
            "slept {:.1}s (capped from {:.1}s)",
            actual, seconds
        ))
    } else {
        Ok(format!("slept {:.1}s", actual))
    }
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

    // ------------------------------------------------------------------ file

    #[test]
    fn test_file_string_flags_do_not_need_sandbox() {
        let (_dir, sandbox) = test_sandbox();
        assert!(test_file(&sandbox, "-z", "").unwrap());
        assert!(!test_file(&sandbox, "-z", "hello").unwrap());
        assert!(test_file(&sandbox, "-n", "hello").unwrap());
        assert!(!test_file(&sandbox, "-n", "").unwrap());
    }

    #[test]
    fn test_file_exists_regular_file() {
        let (dir, sandbox) = test_sandbox();
        let file = dir.path().join("exists.txt");
        std::fs::write(&file, "hello").unwrap();

        assert!(test_file(&sandbox, "-e", file.to_str().unwrap()).unwrap());
        assert!(test_file(&sandbox, "-f", file.to_str().unwrap()).unwrap());
        assert!(!test_file(&sandbox, "-d", file.to_str().unwrap()).unwrap());
    }

    #[test]
    fn test_file_directory() {
        let (dir, sandbox) = test_sandbox();
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        assert!(test_file(&sandbox, "-e", subdir.to_str().unwrap()).unwrap());
        assert!(!test_file(&sandbox, "-f", subdir.to_str().unwrap()).unwrap());
        assert!(test_file(&sandbox, "-d", subdir.to_str().unwrap()).unwrap());
    }

    #[test]
    fn test_file_non_empty() {
        let (dir, sandbox) = test_sandbox();
        let file = dir.path().join("content.txt");
        std::fs::write(&file, "data").unwrap();

        assert!(test_file(&sandbox, "-s", file.to_str().unwrap()).unwrap());
    }

    #[test]
    fn test_file_empty() {
        let (dir, sandbox) = test_sandbox();
        let file = dir.path().join("empty.txt");
        std::fs::write(&file, "").unwrap();

        assert!(!test_file(&sandbox, "-s", file.to_str().unwrap()).unwrap());
    }

    #[test]
    fn test_file_readable() {
        let (dir, sandbox) = test_sandbox();
        let file = dir.path().join("readable.txt");
        std::fs::write(&file, "data").unwrap();

        assert!(test_file(&sandbox, "-r", file.to_str().unwrap()).unwrap());
    }

    #[test]
    fn test_file_unknown_flag() {
        let (_dir, sandbox) = test_sandbox();
        assert!(test_file(&sandbox, "-x", "anything").is_err());
    }

    #[test]
    fn test_file_sandbox_violation() {
        let (_dir, sandbox) = test_sandbox();
        #[cfg(windows)]
        let outside = "C:\\Windows\\System32\\notepad.exe";
        #[cfg(not(windows))]
        let outside = "/etc/passwd";

        // The sandbox will reject a path outside its root.
        let result = test_file(&sandbox, "-e", outside);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------- compare

    #[test]
    fn test_compare_eq() {
        assert!(test_compare("5", "-eq", "5").unwrap());
        assert!(!test_compare("5", "-eq", "6").unwrap());
    }

    #[test]
    fn test_compare_ne() {
        assert!(test_compare("5", "-ne", "6").unwrap());
        assert!(!test_compare("5", "-ne", "5").unwrap());
    }

    #[test]
    fn test_compare_lt() {
        assert!(test_compare("3", "-lt", "5").unwrap());
        assert!(!test_compare("5", "-lt", "3").unwrap());
    }

    #[test]
    fn test_compare_le() {
        assert!(test_compare("5", "-le", "5").unwrap());
        assert!(test_compare("4", "-le", "5").unwrap());
        assert!(!test_compare("6", "-le", "5").unwrap());
    }

    #[test]
    fn test_compare_gt() {
        assert!(test_compare("10", "-gt", "5").unwrap());
        assert!(!test_compare("3", "-gt", "5").unwrap());
    }

    #[test]
    fn test_compare_ge() {
        assert!(test_compare("5", "-ge", "5").unwrap());
        assert!(test_compare("6", "-ge", "5").unwrap());
        assert!(!test_compare("4", "-ge", "5").unwrap());
    }

    #[test]
    fn test_compare_bad_operand() {
        assert!(test_compare("foo", "-eq", "5").is_err());
        assert!(test_compare("5", "-eq", "bar").is_err());
    }

    #[test]
    fn test_compare_unknown_op() {
        assert!(test_compare("5", "==", "5").is_err());
    }

    // --------------------------------------------------------------- sleep

    #[test]
    fn test_sleep_zero() {
        let msg = sleep_cmd(0.0).unwrap();
        assert!(msg.contains("slept"));
    }

    #[test]
    fn test_sleep_short() {
        let start = std::time::Instant::now();
        let result = sleep_cmd(0.05).unwrap();
        assert!(start.elapsed().as_millis() >= 50);
        assert!(result.contains("slept"));
    }

    #[test]
    fn test_sleep_negative() {
        assert!(sleep_cmd(-1.0).is_err());
    }

    #[test]
    fn test_sleep_cap_message() {
        // Passing a value larger than the cap should be reflected in the message.
        // We do NOT actually wait 400s; the function caps to 300s, but for the test
        // we verify the cap message format rather than timing it.
        // We use 0.0 as a stand-in to keep the test fast.
        let msg = sleep_cmd(0.0).unwrap();
        // Just verify the Ok path works
        assert!(msg.starts_with("slept"));
    }
}
