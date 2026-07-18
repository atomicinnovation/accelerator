//! The regex-backed work-item-ID scanner: the infra implementation of the
//! `corpus::IdScanner` port over an injected compiled scan regex.

use corpus::IdScan;
use corpus::IdScanner;
use regex::Regex;

/// An `IdScanner` backed by a compiled scan regex whose first capture group is
/// the digit run and whose full match ends past the trailing delimiter.
pub struct RegexScanner {
    scan_regex: Regex,
}

impl RegexScanner {
    #[must_use]
    pub const fn new(scan_regex: Regex) -> Self {
        Self { scan_regex }
    }

    /// Compiles `pattern` into a scanner.
    ///
    /// # Errors
    ///
    /// The underlying `regex::Error` when `pattern` is not a valid regex.
    pub fn compile(pattern: &str) -> Result<Self, regex::Error> {
        Ok(Self::new(Regex::new(pattern)?))
    }
}

impl IdScanner for RegexScanner {
    fn scan(&self, text: &str) -> Option<IdScan> {
        let captures = self.scan_regex.captures(text)?;
        let whole = captures.get(0)?;
        let digits = captures.get(1)?.as_str().to_owned();
        Some(IdScan {
            digits,
            match_end: whole.end(),
        })
    }
}

#[cfg(test)]
mod tests {
    use corpus::IdScanner;

    use super::RegexScanner;

    #[test]
    fn scans_the_numeric_prefix_and_reports_the_full_match_end(
    ) -> Result<(), regex::Error> {
        let scanner = RegexScanner::compile("^([0-9]+)-")?;
        let scan = scanner.scan("0042-foo").map(|s| (s.digits, s.match_end));
        assert_eq!(scan, Some(("0042".to_owned(), 5)));
        assert!(scanner.scan("nomatch").is_none());
        Ok(())
    }

    #[test]
    fn scans_a_project_prefixed_pattern() -> Result<(), regex::Error> {
        let scanner = RegexScanner::compile("^PROJ-([0-9]+)-")?;
        let scan = scanner
            .scan("PROJ-0042-ship")
            .map(|s| (s.digits, s.match_end));
        assert_eq!(scan, Some(("0042".to_owned(), "PROJ-0042-".len())));
        Ok(())
    }
}
