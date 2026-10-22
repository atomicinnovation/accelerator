//! Crash residue the bootstrap leaves at the top of a cache root: probe files
//! and the lock directories a killed bootstrap never released.

/// The pid a bootstrap wrote, as `printf '%s\n' "$$"` writes it.
///
/// Only an integer of 1 or more parses: `kill` reads 0 and negative values as
/// process groups.
#[must_use]
pub fn parse_pid(contents: &str) -> Option<i32> {
    let digits = contents.trim_end_matches(|c: char| c.is_ascii_whitespace());
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok().filter(|pid| *pid >= 1)
}

#[cfg(test)]
mod tests {
    use super::parse_pid;

    #[test]
    fn a_pid_written_by_the_bootstrap_parses() {
        assert_eq!(parse_pid("4242\n"), Some(4242));
        assert_eq!(parse_pid("4242"), Some(4242));
        assert_eq!(parse_pid("4242 \r\n"), Some(4242));
    }

    #[test]
    fn only_a_positive_decimal_integer_is_a_pid() {
        for contents in [
            "",
            "\n",
            "0",
            "-1",
            "+5",
            "abc",
            " 42",
            "4 2",
            "0x10",
            "99999999999",
        ] {
            assert_eq!(parse_pid(contents), None, "{contents:?}");
        }
    }
}
