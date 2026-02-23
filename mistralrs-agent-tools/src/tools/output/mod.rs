//! Output utilities module.
//!
//! Implements output generation utilities:
//! - echo: Display text
//! - printf: Format and print data
//! - yes: Repeatedly output a string
//! - true_cmd: Return success (exit code 0)
//! - false_cmd: Return failure (exit code 1)

use crate::types::AgentResult;

/// Echo - display a space-joined list of arguments followed by a newline.
///
/// # Examples
///
/// ```
/// use mistralrs_agent_tools::tools::output::echo;
/// assert_eq!(echo(&["hello", "world"]).unwrap(), "hello world");
/// ```
pub fn echo(args: &[&str]) -> AgentResult<String> {
    Ok(args.join(" "))
}

/// Echo without trailing newline - identical to [`echo`] in return value;
/// callers that want raw output without appending a newline should omit it
/// themselves when displaying the result.
///
/// # Examples
///
/// ```
/// use mistralrs_agent_tools::tools::output::echo_n;
/// assert_eq!(echo_n(&["no", "newline"]).unwrap(), "no newline");
/// ```
pub fn echo_n(args: &[&str]) -> AgentResult<String> {
    Ok(args.join(" "))
}

/// Printf - format and print data (simplified subset of POSIX printf).
///
/// Supports `%s` and `%d` placeholders (replaced left-to-right) and the
/// escape sequences `\n`, `\t`, and `\\`.
///
/// # Examples
///
/// ```
/// use mistralrs_agent_tools::tools::output::printf;
/// assert_eq!(
///     printf("Hello %s, you are %d years old", &["world", "42"]).unwrap(),
///     "Hello world, you are 42 years old"
/// );
/// assert_eq!(printf("line1\\nline2", &[]).unwrap(), "line1\nline2");
/// ```
pub fn printf(format: &str, args: &[&str]) -> AgentResult<String> {
    let mut result = format.to_string();

    for arg in args {
        if let Some(pos) = result.find("%s") {
            result.replace_range(pos..pos + 2, arg);
        } else if let Some(pos) = result.find("%d") {
            result.replace_range(pos..pos + 2, arg);
        }
    }

    // Handle common escape sequences
    result = result
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\\", "\\");

    Ok(result)
}

/// Yes - output `text` (default `"y"`) repeated `count` times, one per line.
///
/// # Examples
///
/// ```
/// use mistralrs_agent_tools::tools::output::yes;
/// assert_eq!(yes(None, 3).unwrap(), "y\ny\ny");
/// assert_eq!(yes(Some("ok"), 2).unwrap(), "ok\nok");
/// ```
pub fn yes(text: Option<&str>, count: usize) -> AgentResult<String> {
    let word = text.unwrap_or("y");
    let lines: Vec<&str> = std::iter::repeat_n(word, count).collect();
    Ok(lines.join("\n"))
}

/// True - always returns `Ok(0)` (success exit code).
///
/// # Examples
///
/// ```
/// use mistralrs_agent_tools::tools::output::true_cmd;
/// assert_eq!(true_cmd().unwrap(), 0);
/// ```
pub fn true_cmd() -> AgentResult<i32> {
    Ok(0)
}

/// False - always returns `Ok(1)` (failure exit code).
///
/// # Examples
///
/// ```
/// use mistralrs_agent_tools::tools::output::false_cmd;
/// assert_eq!(false_cmd().unwrap(), 1);
/// ```
pub fn false_cmd() -> AgentResult<i32> {
    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo() {
        assert_eq!(echo(&["hello", "world"]).unwrap(), "hello world");
    }

    #[test]
    fn test_echo_empty() {
        assert_eq!(echo(&[]).unwrap(), "");
    }

    #[test]
    fn test_echo_single() {
        assert_eq!(echo(&["only"]).unwrap(), "only");
    }

    #[test]
    fn test_echo_n() {
        assert_eq!(echo_n(&["no", "newline"]).unwrap(), "no newline");
    }

    #[test]
    fn test_printf_placeholders() {
        assert_eq!(
            printf("Hello %s, you are %d", &["world", "42"]).unwrap(),
            "Hello world, you are 42"
        );
    }

    #[test]
    fn test_printf_escapes() {
        assert_eq!(printf("line1\\nline2", &[]).unwrap(), "line1\nline2");
        assert_eq!(printf("col1\\tcol2", &[]).unwrap(), "col1\tcol2");
        assert_eq!(printf("back\\\\slash", &[]).unwrap(), "back\\slash");
    }

    #[test]
    fn test_printf_no_args() {
        assert_eq!(printf("no placeholders", &[]).unwrap(), "no placeholders");
    }

    #[test]
    fn test_yes_default() {
        assert_eq!(yes(None, 3).unwrap(), "y\ny\ny");
    }

    #[test]
    fn test_yes_custom() {
        assert_eq!(yes(Some("ok"), 2).unwrap(), "ok\nok");
    }

    #[test]
    fn test_yes_zero() {
        assert_eq!(yes(None, 0).unwrap(), "");
    }

    #[test]
    fn test_true_false() {
        assert_eq!(true_cmd().unwrap(), 0);
        assert_eq!(false_cmd().unwrap(), 1);
    }
}
