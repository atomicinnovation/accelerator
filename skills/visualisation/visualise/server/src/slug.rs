use crate::docs::DocTypeKey;

pub fn derive(kind: DocTypeKey, filename: &str) -> Option<String> {
    if !filename.ends_with(".md") {
        return None;
    }
    let stem = &filename[..filename.len() - 3];

    match kind {
        DocTypeKey::Decisions => strip_prefix_numbered(stem, "ADR-"),
        DocTypeKey::Tickets => strip_prefix_ticket_number(stem),
        DocTypeKey::Plans
        | DocTypeKey::Research
        | DocTypeKey::Validations
        | DocTypeKey::Notes
        | DocTypeKey::Prs => strip_prefix_date(stem),
        DocTypeKey::PlanReviews | DocTypeKey::PrReviews => {
            let without_date = strip_prefix_date(stem)?;
            strip_suffix_review_n(&without_date)
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

fn strip_prefix_ticket_number(stem: &str) -> Option<String> {
    let dash = stem.find('-')?;
    let (digits, tail) = stem.split_at(dash);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(tail[1..].to_string()).filter(|s| !s.is_empty())
}

fn strip_prefix_date(stem: &str) -> Option<String> {
    if stem.len() < 11 {
        return None;
    }
    let (head, tail) = stem.split_at(10);
    let bytes = head.as_bytes();
    let ok = bytes.len() == 10
        && bytes[0..4].iter().all(|b| b.is_ascii_digit())
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(|b| b.is_ascii_digit());
    if !ok {
        return None;
    }
    if !tail.starts_with('-') {
        return None;
    }
    Some(tail[1..].to_string()).filter(|s| !s.is_empty())
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
            let got = derive(DocTypeKey::Decisions, input);
            assert_eq!(got.as_deref(), *expected, "input={input}");
        }
    }

    #[test]
    fn tickets_strip_numeric_prefix() {
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
            let got = derive(DocTypeKey::Tickets, input);
            assert_eq!(got.as_deref(), *expected, "input={input}");
        }
    }

    #[test]
    fn dated_types_strip_iso_date() {
        for kind in [
            DocTypeKey::Plans,
            DocTypeKey::Research,
            DocTypeKey::Notes,
            DocTypeKey::Prs,
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
                let got = derive(kind, input);
                assert_eq!(got.as_deref(), *expected, "{kind:?} input={input}");
            }
        }
    }

    #[test]
    fn plan_reviews_strip_date_and_review_n_suffix() {
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
            let got = derive(DocTypeKey::PlanReviews, input);
            assert_eq!(got.as_deref(), *expected, "input={input}");
        }
    }

    #[test]
    fn plan_review_preserves_internal_review_literal() {
        let input = "2026-03-28-initialise-skill-and-review-pr-ephemeral-migration-review-1.md";
        let got = derive(DocTypeKey::PlanReviews, input);
        assert_eq!(
            got.as_deref(),
            Some("initialise-skill-and-review-pr-ephemeral-migration"),
            "internal -review- must be preserved; only the trailing -review-N suffix strips",
        );
    }

    #[test]
    fn plan_review_without_suffix_returns_none() {
        let got = derive(
            DocTypeKey::PlanReviews,
            "2026-04-18-meta-visualiser-phase-2-server-bootstrap.md",
        );
        assert_eq!(got, None);
    }

    #[test]
    fn plan_review_with_non_numeric_suffix_returns_none() {
        let got = derive(DocTypeKey::PlanReviews, "2026-04-18-foo-review-latest.md");
        assert_eq!(got, None);
    }

    #[test]
    fn pr_reviews_use_same_pattern_as_plan_reviews() {
        let input = "2026-04-20-sample-pr-review-3.md";
        assert_eq!(
            derive(DocTypeKey::PrReviews, input).as_deref(),
            Some("sample-pr"),
        );
        let input = "2026-04-20-respond-to-user-feedback-review-1.md";
        assert_eq!(
            derive(DocTypeKey::PrReviews, input).as_deref(),
            Some("respond-to-user-feedback"),
        );
    }

    #[test]
    fn templates_always_return_none() {
        for name in &[
            "adr.md",
            "plan.md",
            "research.md",
            "validation.md",
            "pr-description.md",
        ] {
            assert_eq!(derive(DocTypeKey::Templates, name), None);
        }
    }

    #[test]
    fn non_md_files_return_none_for_every_type() {
        for kind in DocTypeKey::all() {
            assert_eq!(derive(kind, "foo.txt"), None, "{kind:?}");
            assert_eq!(derive(kind, "README.rst"), None, "{kind:?}");
        }
    }
}
