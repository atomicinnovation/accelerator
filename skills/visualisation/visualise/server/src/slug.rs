use crate::config::WorkItemConfig;
use crate::docs::DocTypeKey;

/// Derive the slug for a work-item file using the configured scan regex.
/// The regex must have capture group 1 covering the ID token (digits or
/// project-prefixed digits). The slug is everything after the full match
/// in the filename stem (excluding the `.md` extension).
pub fn derive_work_item_with_regex(
    re: &regex::Regex,
    filename: &str,
) -> Option<String> {
    let stem = filename.strip_suffix(".md")?;
    let m = re.find(stem)?;
    let tail = &stem[m.end()..];
    if tail.is_empty() {
        None
    } else {
        Some(tail.to_string())
    }
}

pub fn derive(
    kind: DocTypeKey,
    filename: &str,
    cfg: &WorkItemConfig,
) -> Option<String> {
    // Exact lowercase `.md` only: paired with the 3-byte slice below and
    // the case-sensitive `.md` checks elsewhere (file_driver list filter,
    // `strip_suffix(".md")`); a case-insensitive rewrite would also accept
    // a bare `.md` filename, changing the empty-stem edge case.
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    if !filename.ends_with(".md") {
        return None;
    }
    let stem = &filename[..filename.len() - 3];

    match kind {
        DocTypeKey::Decisions => strip_prefix_numbered(stem, "ADR-"),
        DocTypeKey::WorkItems => strip_prefix_work_item_id(stem),
        DocTypeKey::Plans
        | DocTypeKey::Research
        | DocTypeKey::Validations
        | DocTypeKey::Notes
        | DocTypeKey::PrDescriptions
        | DocTypeKey::DesignGaps
        | DocTypeKey::DesignInventories => {
            strip_prefix_date_and_optional_id(stem, cfg)
        }
        DocTypeKey::PlanReviews | DocTypeKey::PrReviews => {
            let without_date_and_id =
                strip_prefix_date_and_optional_id(stem, cfg)?;
            strip_suffix_review_n(&without_date_and_id)
        }
        DocTypeKey::WorkItemReviews => {
            // Try the dated shape first (back-compat with old fixtures).
            if let Some(slug) = strip_prefix_date_and_optional_id(stem, cfg)
                .and_then(|s| strip_suffix_review_n(&s))
            {
                return Some(slug);
            }
            // Fall back to the no-date `NNNN-slug-review-N.md` shape used
            // by every file under meta/reviews/work/ today.
            let without_id = strip_optional_work_item_id_prefix(stem, cfg);
            if without_id == stem {
                return None;
            }
            strip_suffix_review_n(without_id)
        }
        DocTypeKey::Templates => None,
    }
}

fn strip_prefix_numbered(stem: &str, prefix: &str) -> Option<String> {
    let rest = stem.strip_prefix(prefix)?;
    let dash = rest.find('-')?;
    let (digits, tail) = rest.split_at(dash);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(tail[1..].to_string()).filter(|s| !s.is_empty())
}

fn strip_prefix_work_item_id(stem: &str) -> Option<String> {
    let dash = stem.find('-')?;
    let (digits, tail) = stem.split_at(dash);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(tail[1..].to_string()).filter(|s| !s.is_empty())
}

fn strip_prefix_date_str(stem: &str) -> Option<&str> {
    if stem.len() < 11 {
        return None;
    }
    let (head, tail) = stem.split_at(10);
    let bytes = head.as_bytes();
    let ok = bytes.len() == 10
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit);
    if !ok {
        return None;
    }
    if !tail.starts_with('-') {
        return None;
    }
    Some(&tail[1..])
}

/// Walk hyphen positions left-to-right. At each candidate boundary, ask
/// the config whether the head is itself a canonical work-item id
/// (strict: exact width, exact prefix). First match wins — shortest
/// valid id prefix.
fn strip_optional_work_item_id_prefix<'a>(
    stem: &'a str,
    cfg: &WorkItemConfig,
) -> &'a str {
    for (i, c) in stem.char_indices() {
        if c == '-' {
            let head = &stem[..i];
            if cfg.is_canonical_id_token(head) {
                return &stem[i + 1..];
            }
        }
    }
    stem
}

fn strip_prefix_date_and_optional_id(
    stem: &str,
    cfg: &WorkItemConfig,
) -> Option<String> {
    let after_date = strip_prefix_date_str(stem)?;
    // If the entire post-date tail is itself a canonical id token (no
    // descriptive tail follows), there's no slug — return None.
    if cfg.is_canonical_id_token(after_date) {
        return None;
    }
    let trimmed = strip_optional_work_item_id_prefix(after_date, cfg);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn strip_suffix_review_n(stem: &str) -> Option<String> {
    let idx = stem.rfind("-review-")?;
    let (head, tail) = stem.split_at(idx);
    let digits = &tail["-review-".len()..];
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(head.to_string()).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decisions_strip_adr_prefix() {
        let cfg = WorkItemConfig::default();
        let cases = &[
            (
                "ADR-0001-context-isolation-principles.md",
                Some("context-isolation-principles"),
            ),
            (
                "ADR-0017-configuration-extension-points.md",
                Some("configuration-extension-points"),
            ),
            ("ADR-12-foo.md", Some("foo")),
            ("0001-context.md", None),
            ("ADR-ABCD-foo.md", None),
            ("ADR-0001-.md", None),
            ("ADR-0001.md", None),
        ];
        for (input, expected) in cases {
            let got = derive(DocTypeKey::Decisions, input, &cfg);
            assert_eq!(got.as_deref(), *expected, "input={input}");
        }
    }

    #[test]
    fn work_items_strip_numeric_prefix() {
        let cfg = WorkItemConfig::default();
        let cases = &[
            (
                "0001-three-layer-review-system-architecture.md",
                Some("three-layer-review-system-architecture"),
            ),
            (
                "0029-template-management-subcommand-surface.md",
                Some("template-management-subcommand-surface"),
            ),
            ("1-short.md", Some("short")),
            ("abc-foo.md", None),
            ("0001.md", None),
        ];
        for (input, expected) in cases {
            let got = derive(DocTypeKey::WorkItems, input, &cfg);
            assert_eq!(got.as_deref(), *expected, "input={input}");
        }
    }

    #[test]
    fn dated_types_strip_iso_date() {
        let cfg = WorkItemConfig::default();
        for kind in [
            DocTypeKey::Plans,
            DocTypeKey::Research,
            DocTypeKey::Notes,
            DocTypeKey::PrDescriptions,
            DocTypeKey::Validations,
        ] {
            let cases = &[
                ("2026-04-17-pr-review-agents.md", Some("pr-review-agents")),
                (
                    "2026-02-22-pr-review-agents-design.md",
                    Some("pr-review-agents-design"),
                ),
                ("20260417-foo.md", None),
                ("2026-4-17-foo.md", None),
                ("2026-04-17-.md", None),
                ("2026-04-17.md", None),
                ("2026-04-17-foo.txt", None),
            ];
            for (input, expected) in cases {
                let got = derive(kind, input, &cfg);
                assert_eq!(got.as_deref(), *expected, "{kind:?} input={input}");
            }
        }
    }

    #[test]
    fn dated_types_strip_optional_work_item_id_after_date() {
        let cfg = WorkItemConfig::default();
        for kind in [
            DocTypeKey::Plans,
            DocTypeKey::Research,
            DocTypeKey::Notes,
            DocTypeKey::PrDescriptions,
            DocTypeKey::Validations,
        ] {
            let cases = &[
                (
                    "2026-05-31-0040-pipeline-visualisation-overhaul.md",
                    Some("pipeline-visualisation-overhaul"),
                ),
                (
                    "2026-05-05-0031-consolidate-accelerator-owned-files.md",
                    Some("consolidate-accelerator-owned-files"),
                ),
                ("2026-02-22-pr-review-agents.md", Some("pr-review-agents")),
                ("2026-04-17-foo-bar.md", Some("foo-bar")),
                ("2026-04-17-100-day-plan.md", Some("100-day-plan")),
                ("2026-05-31-0040-.md", None),
                ("2026-05-31-0040.md", None),
            ];
            for (input, expected) in cases {
                let got = derive(kind, input, &cfg);
                assert_eq!(got.as_deref(), *expected, "{kind:?} input={input}");
            }
        }
    }

    #[test]
    fn dated_types_strip_project_prefixed_work_item_id_after_date() {
        let cfg = WorkItemConfig::with_pattern_for_test("PROJ", 4);
        let cases = &[
            ("2026-05-31-PROJ-0040-pipeline.md", Some("pipeline")),
            // Bare numeric ID does NOT match this pattern; preserve descriptor.
            ("2026-05-31-0040-pipeline.md", Some("0040-pipeline")),
            ("2026-02-22-foo-bar.md", Some("foo-bar")),
        ];
        for (input, expected) in cases {
            let got = derive(DocTypeKey::Plans, input, &cfg);
            assert_eq!(got.as_deref(), *expected, "input={input}");
        }
    }

    #[test]
    fn plan_reviews_strip_date_and_review_n_suffix() {
        let cfg = WorkItemConfig::default();
        let cases = &[
            (
                "2026-04-18-meta-visualiser-phase-2-server-bootstrap-review-1.md",
                Some("meta-visualiser-phase-2-server-bootstrap"),
            ),
            (
                "2026-03-29-template-management-subcommands-review-1.md",
                Some("template-management-subcommands"),
            ),
            ("2026-04-18-foo-review-12.md", Some("foo")),
        ];
        for (input, expected) in cases {
            let got = derive(DocTypeKey::PlanReviews, input, &cfg);
            assert_eq!(got.as_deref(), *expected, "input={input}");
        }
    }

    #[test]
    fn plan_reviews_strip_optional_work_item_id_after_date() {
        let cfg = WorkItemConfig::default();
        let cases = &[
            (
                "2026-05-05-0031-consolidate-accelerator-owned-files-review-1.md",
                Some("consolidate-accelerator-owned-files"),
            ),
            ("2026-04-18-foo-review-1.md", Some("foo")),
            (
                "2026-03-28-initialise-skill-and-review-pr-ephemeral-migration-review-1.md",
                Some("initialise-skill-and-review-pr-ephemeral-migration"),
            ),
        ];
        for (input, expected) in cases {
            let got = derive(DocTypeKey::PlanReviews, input, &cfg);
            assert_eq!(got.as_deref(), *expected, "input={input}");
        }
    }

    #[test]
    fn plan_review_preserves_internal_review_literal() {
        let cfg = WorkItemConfig::default();
        let input = "2026-03-28-initialise-skill-and-review-pr-ephemeral-migration-review-1.md";
        let got = derive(DocTypeKey::PlanReviews, input, &cfg);
        assert_eq!(
            got.as_deref(),
            Some("initialise-skill-and-review-pr-ephemeral-migration"),
            "internal -review- must be preserved; only the trailing -review-N suffix strips",
        );
    }

    #[test]
    fn plan_review_without_suffix_returns_none() {
        let cfg = WorkItemConfig::default();
        let got = derive(
            DocTypeKey::PlanReviews,
            "2026-04-18-meta-visualiser-phase-2-server-bootstrap.md",
            &cfg,
        );
        assert_eq!(got, None);
    }

    #[test]
    fn plan_review_with_non_numeric_suffix_returns_none() {
        let cfg = WorkItemConfig::default();
        let got = derive(
            DocTypeKey::PlanReviews,
            "2026-04-18-foo-review-latest.md",
            &cfg,
        );
        assert_eq!(got, None);
    }

    #[test]
    fn work_item_reviews_strip_date_and_review_n_suffix() {
        let cfg = WorkItemConfig::default();
        let cases = &[
            (
                "2026-04-30-completeness-pass-review-1.md",
                Some("completeness-pass"),
            ),
            ("2026-05-02-foo-review-7.md", Some("foo")),
            ("2026-04-30-no-suffix.md", None),
        ];
        for (input, expected) in cases {
            let got = derive(DocTypeKey::WorkItemReviews, input, &cfg);
            assert_eq!(got.as_deref(), *expected, "input={input}");
        }
    }

    #[test]
    fn work_item_reviews_accept_no_date_id_prefixed_shape() {
        let cfg = WorkItemConfig::default();
        let cases = &[
            (
                "0030-centralise-path-defaults-review-1.md",
                Some("centralise-path-defaults"),
            ),
            (
                "0061-adr-typed-linkage-vocabulary-review-2.md",
                Some("adr-typed-linkage-vocabulary"),
            ),
            (
                "0001-three-layer-review-system-architecture-review-1.md",
                Some("three-layer-review-system-architecture"),
            ),
            ("0040-final-review-review-1.md", Some("final-review")),
        ];
        for (input, expected) in cases {
            let got = derive(DocTypeKey::WorkItemReviews, input, &cfg);
            assert_eq!(got.as_deref(), *expected, "input={input}");
        }
    }

    #[test]
    fn work_item_reviews_dated_shape_still_accepted_for_back_compat() {
        let cfg = WorkItemConfig::default();
        let cases = &[
            (
                "2026-04-30-completeness-pass-review-1.md",
                Some("completeness-pass"),
            ),
            ("2026-05-02-foo-review-7.md", Some("foo")),
        ];
        for (input, expected) in cases {
            let got = derive(DocTypeKey::WorkItemReviews, input, &cfg);
            assert_eq!(got.as_deref(), *expected, "input={input}");
        }
    }

    #[test]
    fn pr_reviews_use_same_pattern_as_plan_reviews() {
        let cfg = WorkItemConfig::default();
        let input = "2026-04-20-sample-pr-review-3.md";
        assert_eq!(
            derive(DocTypeKey::PrReviews, input, &cfg).as_deref(),
            Some("sample-pr"),
        );
        let input = "2026-04-20-respond-to-user-feedback-review-1.md";
        assert_eq!(
            derive(DocTypeKey::PrReviews, input, &cfg).as_deref(),
            Some("respond-to-user-feedback"),
        );
    }

    #[test]
    fn templates_always_return_none() {
        let cfg = WorkItemConfig::default();
        for name in &[
            "adr.md",
            "plan.md",
            "research.md",
            "validation.md",
            "pr-description.md",
        ] {
            assert_eq!(derive(DocTypeKey::Templates, name, &cfg), None);
        }
    }

    #[test]
    fn non_md_files_return_none_for_every_type() {
        let cfg = WorkItemConfig::default();
        for kind in DocTypeKey::all() {
            assert_eq!(derive(kind, "foo.txt", &cfg), None, "{kind:?}");
            assert_eq!(derive(kind, "README.rst", &cfg), None, "{kind:?}");
        }
    }

    fn compile_scan(pattern: &str, project_code: &str) -> regex::Regex {
        let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../skills/work/scripts/work-item-pattern.sh");
        let out = std::process::Command::new(&script)
            .arg("--compile-scan")
            .arg(pattern)
            .arg(project_code)
            .output()
            .expect("work-item-pattern.sh must be executable");
        assert!(
            out.status.success(),
            "compile-scan failed for pattern={pattern}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        regex::Regex::new(String::from_utf8(out.stdout).unwrap().trim())
            .expect("compiler must produce valid regex")
    }

    fn compile_scan_status(
        pattern: &str,
        project_code: &str,
    ) -> std::process::Output {
        let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../skills/work/scripts/work-item-pattern.sh");
        std::process::Command::new(&script)
            .arg("--compile-scan")
            .arg(pattern)
            .arg(project_code)
            .output()
            .expect("work-item-pattern.sh must be executable")
    }

    #[test]
    fn work_items_with_project_pattern_strip_full_id_prefix() {
        let re = compile_scan("{project}-{number:04d}", "PROJ");
        let cases = &[
            ("PROJ-0042-ship-the-thing.md", Some("ship-the-thing")),
            ("PROJ-1-short.md", Some("short")),
            ("PROJ-0042.md", None),
            ("malformed.md", None),
        ];
        for (input, expected) in cases {
            let got = derive_work_item_with_regex(&re, input);
            assert_eq!(got.as_deref(), *expected, "input={input}");
        }
    }

    #[test]
    fn work_items_with_lowercase_or_digit_project_code() {
        let re = compile_scan("{project}-{number:04d}", "web2");
        assert_eq!(
            derive_work_item_with_regex(&re, "web2-0042-foo.md").as_deref(),
            Some("foo")
        );
    }

    #[test]
    fn work_items_default_numeric_pattern_still_works() {
        let re = compile_scan("{number:04d}", "");
        assert_eq!(
            derive_work_item_with_regex(
                &re,
                "0001-three-layer-review-system-architecture.md"
            )
            .as_deref(),
            Some("three-layer-review-system-architecture")
        );
    }

    #[test]
    fn invalid_id_pattern_fails_compilation_with_clear_message() {
        let out = compile_scan_status("not-a-valid-pattern", "");
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("E_PATTERN_") || stderr.contains("invalid"),
            "stderr should name the failure: {stderr}"
        );
    }
}
