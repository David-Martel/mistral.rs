//! Numeric operations module.
//!
//! Implements numeric and mathematical utilities:
//! - expr: Evaluate simple integer expressions
//! - factor: Print prime factors of a number
//! - seq: Generate sequences of numbers
//! - numfmt: Format numbers with SI/IEC unit prefixes

use crate::types::{AgentError, AgentResult};

/// Expr - evaluate a simple integer expression of the form `VALUE OP VALUE`.
///
/// Supported operators: `+`, `-`, `*`, `/`, `%`.
/// A single token is returned as-is if it is a valid integer.
///
/// # Examples
///
/// ```
/// use mistralrs_agent_tools::tools::numeric::expr;
/// assert_eq!(expr("3 + 4").unwrap(), "7");
/// assert_eq!(expr("10 / 3").unwrap(), "3");
/// ```
pub fn expr(expression: &str) -> AgentResult<String> {
    let tokens: Vec<&str> = expression.split_whitespace().collect();

    match tokens.len() {
        0 => Err(AgentError::InvalidInput("Empty expression".into())),
        1 => {
            let val: i64 = tokens[0]
                .parse()
                .map_err(|_| AgentError::InvalidInput(format!("Not a number: {}", tokens[0])))?;
            Ok(val.to_string())
        }
        3 => {
            let a: i64 = tokens[0]
                .parse()
                .map_err(|_| AgentError::InvalidInput(format!("Not a number: {}", tokens[0])))?;
            let b: i64 = tokens[2]
                .parse()
                .map_err(|_| AgentError::InvalidInput(format!("Not a number: {}", tokens[2])))?;
            let result = match tokens[1] {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                "/" => {
                    if b == 0 {
                        return Err(AgentError::InvalidInput("Division by zero".into()));
                    }
                    a / b
                }
                "%" => {
                    if b == 0 {
                        return Err(AgentError::InvalidInput("Division by zero".into()));
                    }
                    a % b
                }
                op => {
                    return Err(AgentError::InvalidInput(format!(
                        "Unknown operator: {}",
                        op
                    )))
                }
            };
            Ok(result.to_string())
        }
        _ => Err(AgentError::InvalidInput("Expected: VALUE OP VALUE".into())),
    }
}

/// Factor - print the prime factorisation of `n` in GNU coreutils format.
///
/// # Examples
///
/// ```
/// use mistralrs_agent_tools::tools::numeric::factor;
/// assert_eq!(factor(12).unwrap(), "12: 2 2 3");
/// assert_eq!(factor(7).unwrap(), "7: 7");
/// assert_eq!(factor(1).unwrap(), "1:");
/// ```
pub fn factor(n: u64) -> AgentResult<String> {
    if n == 0 {
        return Ok("0:".to_string());
    }
    if n == 1 {
        return Ok("1:".to_string());
    }

    let mut factors: Vec<u64> = Vec::new();
    let mut remaining = n;
    let mut divisor = 2u64;

    while divisor * divisor <= remaining {
        while remaining.is_multiple_of(divisor) {
            factors.push(divisor);
            remaining /= divisor;
        }
        divisor += 1;
    }
    if remaining > 1 {
        factors.push(remaining);
    }

    let factor_strs: Vec<String> = factors.iter().map(|f| f.to_string()).collect();
    Ok(format!("{}: {}", n, factor_strs.join(" ")))
}

/// Seq - generate a sequence of integers from `first` to `last` (inclusive)
/// with the given `increment`.
///
/// Returns each number on its own line. Returns an error if `increment` is zero.
///
/// # Examples
///
/// ```
/// use mistralrs_agent_tools::tools::numeric::seq;
/// assert_eq!(seq(1, 1, 5).unwrap(), "1\n2\n3\n4\n5");
/// assert_eq!(seq(5, -1, 3).unwrap(), "5\n4\n3");
/// ```
pub fn seq(first: i64, increment: i64, last: i64) -> AgentResult<String> {
    if increment == 0 {
        return Err(AgentError::InvalidInput("Increment cannot be zero".into()));
    }

    let mut result: Vec<String> = Vec::new();
    let mut current = first;

    if increment > 0 {
        while current <= last {
            result.push(current.to_string());
            current += increment;
        }
    } else {
        while current >= last {
            result.push(current.to_string());
            current += increment;
        }
    }

    Ok(result.join("\n"))
}

/// Numfmt - format a byte count with a unit prefix.
///
/// `to_unit` may be `"iec"` (powers of 1024, e.g. `1.0K`) or `"si"` (powers
/// of 1000, e.g. `1.0K`).
///
/// # Examples
///
/// ```
/// use mistralrs_agent_tools::tools::numeric::numfmt;
/// assert_eq!(numfmt(1024, "iec").unwrap(), "1.0K");
/// assert_eq!(numfmt(1_000_000, "si").unwrap(), "1.0M");
/// ```
pub fn numfmt(value: u64, to_unit: &str) -> AgentResult<String> {
    match to_unit {
        "iec" => {
            let units = ["", "K", "M", "G", "T", "P"];
            let mut val = value as f64;
            let mut unit_idx = 0usize;
            while val >= 1024.0 && unit_idx < units.len() - 1 {
                val /= 1024.0;
                unit_idx += 1;
            }
            if unit_idx == 0 {
                Ok(format!("{}", value))
            } else {
                Ok(format!("{:.1}{}", val, units[unit_idx]))
            }
        }
        "si" => {
            let units = ["", "K", "M", "G", "T", "P"];
            let mut val = value as f64;
            let mut unit_idx = 0usize;
            while val >= 1000.0 && unit_idx < units.len() - 1 {
                val /= 1000.0;
                unit_idx += 1;
            }
            if unit_idx == 0 {
                Ok(format!("{}", value))
            } else {
                Ok(format!("{:.1}{}", val, units[unit_idx]))
            }
        }
        other => Err(AgentError::InvalidInput(format!(
            "Unknown unit system: {}",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_add() {
        assert_eq!(expr("3 + 4").unwrap(), "7");
    }

    #[test]
    fn test_expr_subtract() {
        assert_eq!(expr("10 - 3").unwrap(), "7");
    }

    #[test]
    fn test_expr_multiply() {
        assert_eq!(expr("6 * 7").unwrap(), "42");
    }

    #[test]
    fn test_expr_divide() {
        assert_eq!(expr("10 / 3").unwrap(), "3");
    }

    #[test]
    fn test_expr_modulo() {
        assert_eq!(expr("10 % 3").unwrap(), "1");
    }

    #[test]
    fn test_expr_single_value() {
        assert_eq!(expr("42").unwrap(), "42");
    }

    #[test]
    fn test_expr_div_zero() {
        assert!(expr("5 / 0").is_err());
        assert!(expr("5 % 0").is_err());
    }

    #[test]
    fn test_expr_empty() {
        assert!(expr("").is_err());
    }

    #[test]
    fn test_expr_bad_operand() {
        assert!(expr("foo + 1").is_err());
    }

    #[test]
    fn test_expr_unknown_op() {
        assert!(expr("1 ^ 2").is_err());
    }

    #[test]
    fn test_factor_composite() {
        assert_eq!(factor(12).unwrap(), "12: 2 2 3");
    }

    #[test]
    fn test_factor_prime() {
        assert_eq!(factor(7).unwrap(), "7: 7");
    }

    #[test]
    fn test_factor_one() {
        assert_eq!(factor(1).unwrap(), "1:");
    }

    #[test]
    fn test_factor_zero() {
        assert_eq!(factor(0).unwrap(), "0:");
    }

    #[test]
    fn test_factor_large_prime() {
        // 97 is prime
        assert_eq!(factor(97).unwrap(), "97: 97");
    }

    #[test]
    fn test_seq_ascending() {
        assert_eq!(seq(1, 1, 5).unwrap(), "1\n2\n3\n4\n5");
    }

    #[test]
    fn test_seq_step() {
        assert_eq!(seq(0, 2, 6).unwrap(), "0\n2\n4\n6");
    }

    #[test]
    fn test_seq_descending() {
        assert_eq!(seq(5, -1, 3).unwrap(), "5\n4\n3");
    }

    #[test]
    fn test_seq_single() {
        assert_eq!(seq(3, 1, 3).unwrap(), "3");
    }

    #[test]
    fn test_seq_empty_range() {
        // first > last with positive increment produces no output
        assert_eq!(seq(5, 1, 3).unwrap(), "");
    }

    #[test]
    fn test_seq_zero_increment() {
        assert!(seq(1, 0, 5).is_err());
    }

    #[test]
    fn test_numfmt_iec_kibibyte() {
        assert_eq!(numfmt(1024, "iec").unwrap(), "1.0K");
    }

    #[test]
    fn test_numfmt_iec_mebibyte() {
        assert_eq!(numfmt(1_048_576, "iec").unwrap(), "1.0M");
    }

    #[test]
    fn test_numfmt_iec_small() {
        assert_eq!(numfmt(500, "iec").unwrap(), "500");
    }

    #[test]
    fn test_numfmt_si_kilobyte() {
        assert_eq!(numfmt(1000, "si").unwrap(), "1.0K");
    }

    #[test]
    fn test_numfmt_si_megabyte() {
        assert_eq!(numfmt(1_000_000, "si").unwrap(), "1.0M");
    }

    #[test]
    fn test_numfmt_unknown_unit() {
        assert!(numfmt(1024, "binary").is_err());
    }
}
