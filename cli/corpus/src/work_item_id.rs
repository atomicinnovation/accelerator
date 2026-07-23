//! The work-item-ID runtime predicate and the injected scan port.
//!
//! The pattern-DSL compiler that turns `work.id_pattern` into a scan regex is a
//! work/config concern; this crate takes the compiled scanner by injection so it
//! never depends on `regex`.

/// A match produced by an [`IdScanner`] over a filename.
pub struct IdScan {
    pub digits: String,
    pub match_end: usize,
}

/// Recognises a work-item-ID prefix in a filename. Implemented in the adapter
/// layer over the compiled scan regex.
pub trait IdScanner {
    fn scan(&self, text: &str) -> Option<IdScan>;
}

/// The identity scheme a workspace configures for its work items.
#[derive(Debug, Clone)]
pub struct WorkItemIdScheme {
    pub id_pattern: String,
    pub default_project_code: Option<String>,
}

impl Default for WorkItemIdScheme {
    fn default() -> Self {
        Self::numeric()
    }
}

impl WorkItemIdScheme {
    #[must_use]
    pub fn numeric() -> Self {
        Self {
            id_pattern: "{number:04d}".to_owned(),
            default_project_code: None,
        }
    }

    /// True iff `token` is exactly a canonical work-item-ID under this scheme:
    /// the correct project prefix (if any) and the exact configured digit width
    /// (or any non-empty digit run when the width is unspecified).
    #[must_use]
    pub fn is_canonical_id_token(&self, token: &str) -> bool {
        let width = self.canonical_digit_width();
        let digits = match &self.default_project_code {
            Some(code) => match token.strip_prefix(&format!("{code}-")) {
                Some(rest) => rest,
                None => return false,
            },
            None => token,
        };
        if width == 0 {
            !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
        } else {
            digits.len() == width && digits.chars().all(|c| c.is_ascii_digit())
        }
    }

    /// The zero-padded digit width from the pattern's `{number:0Nd}` segment, or
    /// `0` (admit any width) when unspecified.
    #[must_use]
    pub fn canonical_digit_width(&self) -> usize {
        let pattern = &self.id_pattern;
        let Some(start) = pattern.find("{number") else {
            return 0;
        };
        let rest = &pattern[start + "{number".len()..];
        let Some(end) = rest.find('}') else {
            return 0;
        };
        let spec = rest[..end].trim_start_matches(':').trim_end_matches('d');
        if spec.is_empty() {
            return 0;
        }
        let digits = spec.trim_start_matches('0');
        if digits.is_empty() {
            return 0;
        }
        digits.parse::<usize>().unwrap_or(0)
    }

    /// Validates and normalises a work-item-ID from any source. A prefixed form
    /// passes through verbatim; bare digits gain the configured project code.
    #[must_use]
    pub fn normalise_id(&self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        if let Some((prefix, digits)) = trimmed.split_once('-') {
            if prefix.is_empty()
                || !prefix.chars().all(|c| c.is_ascii_alphabetic())
                || digits.is_empty()
                || !digits.chars().all(|c| c.is_ascii_digit())
            {
                return None;
            }
            return Some(trimmed.to_owned());
        }
        if !trimmed.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(self.default_project_code.as_ref().map_or_else(
            || trimmed.to_owned(),
            |code| format!("{code}-{trimmed}"),
        ))
    }

    /// Extracts the full work-item-ID from a filename: the injected `scanner`
    /// supplies the primary digit run; a bare-numeric fallback keys legacy files
    /// when a project code is configured.
    #[must_use]
    pub fn extract_id(
        &self,
        filename: &str,
        scanner: &dyn IdScanner,
    ) -> Option<String> {
        if let Some(scan) = scanner.scan(filename) {
            let digits = scan.digits;
            return Some(match &self.default_project_code {
                Some(code) => format!("{code}-{digits}"),
                None => digits,
            });
        }
        let code = self.default_project_code.as_deref()?;
        let dash = filename.find('-')?;
        let prefix = &filename[..dash];
        if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(format!("{code}-{prefix}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{IdScan, IdScanner, WorkItemIdScheme};

    struct DigitRunScanner;

    impl IdScanner for DigitRunScanner {
        fn scan(&self, text: &str) -> Option<IdScan> {
            let digits: String =
                text.chars().take_while(char::is_ascii_digit).collect();
            if digits.is_empty() || !text[digits.len()..].starts_with('-') {
                return None;
            }
            let match_end = digits.len() + 1;
            Some(IdScan { digits, match_end })
        }
    }

    fn project(code: &str, width: usize) -> WorkItemIdScheme {
        WorkItemIdScheme {
            id_pattern: format!("{{project}}-{{number:0{width}d}}"),
            default_project_code: Some(code.to_owned()),
        }
    }

    #[test]
    fn canonical_digit_width_reads_the_pattern() {
        assert_eq!(WorkItemIdScheme::numeric().canonical_digit_width(), 4);
        let any = WorkItemIdScheme {
            id_pattern: "{number}".to_owned(),
            default_project_code: None,
        };
        assert_eq!(any.canonical_digit_width(), 0);
        let admit_any = WorkItemIdScheme {
            id_pattern: "{number:0d}".to_owned(),
            default_project_code: None,
        };
        assert_eq!(admit_any.canonical_digit_width(), 0);
    }

    #[test]
    fn is_canonical_id_token_under_default_numeric() {
        let scheme = WorkItemIdScheme::numeric();
        assert!(scheme.is_canonical_id_token("0040"));
        assert!(!scheme.is_canonical_id_token("40"));
        assert!(!scheme.is_canonical_id_token("00040"));
        assert!(!scheme.is_canonical_id_token("100"));
        assert!(!scheme.is_canonical_id_token("004A"));
        assert!(!scheme.is_canonical_id_token(""));
    }

    #[test]
    fn is_canonical_id_token_under_project_pattern() {
        let scheme = project("PROJ", 4);
        assert!(scheme.is_canonical_id_token("PROJ-0040"));
        assert!(!scheme.is_canonical_id_token("0040"));
        assert!(!scheme.is_canonical_id_token("PROJ-40"));
        assert!(!scheme.is_canonical_id_token("OTHER-0040"));
    }

    #[test]
    fn normalise_id_handles_prefixed_and_bare_forms() {
        let numeric = WorkItemIdScheme::numeric();
        assert_eq!(
            numeric.normalise_id("ENG-0042").as_deref(),
            Some("ENG-0042")
        );
        assert_eq!(numeric.normalise_id("0042").as_deref(), Some("0042"));
        assert_eq!(numeric.normalise_id("ENG0042"), None);
        assert_eq!(numeric.normalise_id("PROJ-1.2"), None);
        assert_eq!(numeric.normalise_id("  0042  ").as_deref(), Some("0042"));

        let eng = project("ENG", 4);
        assert_eq!(eng.normalise_id("42").as_deref(), Some("ENG-42"));
        assert_eq!(eng.normalise_id("OPS-7").as_deref(), Some("OPS-7"));
    }

    #[test]
    fn extract_id_uses_the_scanner_then_the_fallback() {
        let numeric = WorkItemIdScheme::numeric();
        assert_eq!(
            numeric
                .extract_id("0042-foo.md", &DigitRunScanner)
                .as_deref(),
            Some("0042")
        );
        assert_eq!(numeric.extract_id("malformed.md", &DigitRunScanner), None);

        let proj = project("PROJ", 4);
        assert_eq!(
            proj.extract_id("0042-legacy.md", &DigitRunScanner)
                .as_deref(),
            Some("PROJ-0042")
        );
    }
}
