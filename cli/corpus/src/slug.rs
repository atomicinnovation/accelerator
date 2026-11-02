//! Per-doc-type slug derivation and the canonical stem humaniser.

use crate::doc_type::DocTypeKey;
use crate::id_convention::IdConvention;

/// Derives the descriptive slug for a document filename, dispatching on its
/// type. Returns `None` when the filename carries no slug.
#[must_use]
pub fn derive(
    kind: DocTypeKey,
    filename: &str,
    ids: &dyn IdConvention,
) -> Option<String> {
    let stem = markdown_stem(filename)?;
    match kind {
        DocTypeKey::Decisions => strip_prefix_numbered(stem, "ADR-"),
        DocTypeKey::WorkItems => derive_work_item(filename, ids)
            .or_else(|| strip_prefix_work_item_id(stem)),
        DocTypeKey::Plans
        | DocTypeKey::Research
        | DocTypeKey::Validations
        | DocTypeKey::Notes
        | DocTypeKey::PrDescriptions
        | DocTypeKey::DesignGaps
        | DocTypeKey::DesignInventories
        | DocTypeKey::RootCauseAnalyses => {
            strip_prefix_date_and_optional_id(stem, ids)
        }
        DocTypeKey::PlanReviews | DocTypeKey::PrReviews => {
            strip_prefix_date_and_optional_id(stem, ids)
                .and_then(|slug| strip_suffix_review_n(&slug))
        }
        DocTypeKey::WorkItemReviews => derive_work_item_review(stem, ids),
        DocTypeKey::TopicResearch => {
            Some(stem.to_owned()).filter(|slug| !slug.is_empty())
        }
        DocTypeKey::Templates => None,
    }
}

/// Derives a work-item slug: the tail after the convention's `slug_start`.
#[must_use]
pub fn derive_work_item(
    filename: &str,
    ids: &dyn IdConvention,
) -> Option<String> {
    let stem = markdown_stem(filename)?;
    let tail = &stem[ids.slug_start(stem)?..];
    if tail.is_empty() {
        None
    } else {
        Some(tail.to_owned())
    }
}

fn markdown_stem(filename: &str) -> Option<&str> {
    let (stem, extension) = filename.rsplit_once('.')?;
    extension.eq_ignore_ascii_case("md").then_some(stem)
}

fn derive_work_item_review(
    stem: &str,
    ids: &dyn IdConvention,
) -> Option<String> {
    if let Some(slug) = strip_prefix_date_and_optional_id(stem, ids)
        .and_then(|dated| strip_suffix_review_n(&dated))
    {
        return Some(slug);
    }
    let without_id = strip_optional_work_item_id_prefix(stem, ids);
    if without_id == stem {
        return None;
    }
    strip_suffix_review_n(without_id)
}

fn strip_prefix_numbered(stem: &str, prefix: &str) -> Option<String> {
    let rest = stem.strip_prefix(prefix)?;
    let dash = rest.find('-')?;
    let (digits, tail) = rest.split_at(dash);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(tail[1..].to_owned()).filter(|slug| !slug.is_empty())
}

fn strip_prefix_work_item_id_str(stem: &str) -> Option<&str> {
    let dash = stem.find('-')?;
    let (digits, tail) = stem.split_at(dash);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let rest = &tail[1..];
    if rest.is_empty() {
        None
    } else {
        Some(rest)
    }
}

fn strip_prefix_work_item_id(stem: &str) -> Option<String> {
    strip_prefix_work_item_id_str(stem).map(str::to_owned)
}

fn is_iso_date_prefix(stem: &str) -> bool {
    let bytes = stem.as_bytes();
    bytes.len() >= 10
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit)
}

fn strip_prefix_date_str(stem: &str) -> Option<&str> {
    if stem.len() < 11 || !is_iso_date_prefix(stem) {
        return None;
    }
    let tail = &stem[10..];
    tail.strip_prefix('-')
}

fn strip_optional_work_item_id_prefix<'a>(
    stem: &'a str,
    ids: &dyn IdConvention,
) -> &'a str {
    for (index, character) in stem.char_indices() {
        if character == '-' && ids.is_id(&stem[..index]) {
            return &stem[index + 1..];
        }
    }
    stem
}

fn strip_prefix_date_and_optional_id(
    stem: &str,
    ids: &dyn IdConvention,
) -> Option<String> {
    let after_date = strip_prefix_date_str(stem)?;
    if ids.is_id(after_date) {
        return None;
    }
    let trimmed = strip_optional_work_item_id_prefix(after_date, ids);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn strip_suffix_review_n(stem: &str) -> Option<String> {
    let index = stem.rfind("-review-")?;
    let (head, tail) = stem.split_at(index);
    let digits = &tail["-review-".len()..];
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(head.to_owned()).filter(|slug| !slug.is_empty())
}

/// Humanises a filename stem for display: strips one leading date or ID prefix,
/// splits on `-`, and title-cases each segment.
#[must_use]
pub fn humanise_slug(stem: &str) -> String {
    strip_humanise_prefix(stem)
        .split('-')
        .filter(|segment| !segment.is_empty())
        .map(title_case_segment)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Strips one leading date or work-item-ID prefix from a stem.
#[must_use]
pub fn strip_humanise_prefix(stem: &str) -> &str {
    if let Some(rest) = strip_prefix_date_str(stem) {
        if !rest.is_empty() {
            return rest;
        }
    }
    if is_iso_date_prefix(stem) {
        return stem;
    }
    if let Some(rest) = strip_prefix_work_item_id_str(stem) {
        return rest;
    }
    stem
}

/// The canonical title-caser.
///
/// The server's `config::label_from_key` and `api::library::humanise_status` are
/// byte-identical copies that retire onto this one when 0168 folds the server
/// into the workspace, so it is public rather than crate-private: they cannot
/// import what they cannot see.
#[must_use]
pub fn title_case_segment(segment: &str) -> String {
    let mut chars = segment.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + chars.as_str()
    })
}

#[cfg(test)]
mod tests {
    use crate::doc_type::DocTypeKey;
    use crate::id_convention::test_ids::{FourDigitIds, NUMERIC};

    use super::{derive, humanise_slug};

    const PROJECT: FourDigitIds = FourDigitIds { key: Some("PROJ") };

    #[test]
    fn decisions_strip_the_adr_prefix() {
        let cases = [
            ("ADR-0001-context-isolation.md", Some("context-isolation")),
            ("ADR-12-foo.md", Some("foo")),
            ("0001-context.md", None),
            ("ADR-0001.md", None),
        ];
        for (input, expected) in cases {
            assert_eq!(
                derive(DocTypeKey::Decisions, input, &NUMERIC).as_deref(),
                expected,
                "{input}"
            );
        }
    }

    #[test]
    fn work_items_strip_the_numeric_id() {
        assert_eq!(
            derive(
                DocTypeKey::WorkItems,
                "0029-template-subcommand.md",
                &NUMERIC
            )
            .as_deref(),
            Some("template-subcommand")
        );
        assert_eq!(derive(DocTypeKey::WorkItems, "0001.md", &NUMERIC), None);
    }

    #[test]
    fn work_items_keep_their_slug_under_an_upper_case_extension() {
        assert_eq!(
            derive(DocTypeKey::WorkItems, "0400-probe.MD", &NUMERIC).as_deref(),
            Some("probe")
        );
    }

    #[test]
    fn work_items_fall_back_to_bare_numeric_under_a_project_pattern() {
        assert_eq!(
            derive(DocTypeKey::WorkItems, "PROJ-0042-ship.md", &PROJECT)
                .as_deref(),
            Some("ship")
        );
        assert_eq!(
            derive(DocTypeKey::WorkItems, "0042-legacy.md", &PROJECT)
                .as_deref(),
            Some("legacy")
        );
    }

    #[test]
    fn dated_types_strip_the_date_and_optional_id() {
        let cases = [
            ("2026-04-17-pr-review-agents.md", Some("pr-review-agents")),
            (
                "2026-05-31-0040-pipeline-overhaul.md",
                Some("pipeline-overhaul"),
            ),
            ("2026-04-17-100-day-plan.md", Some("100-day-plan")),
            ("2026-05-31-0040.md", None),
            ("2026-04-17.md", None),
        ];
        for (input, expected) in cases {
            assert_eq!(
                derive(DocTypeKey::Plans, input, &NUMERIC).as_deref(),
                expected,
                "{input}"
            );
        }
    }

    #[test]
    fn plan_reviews_strip_the_trailing_review_suffix() {
        assert_eq!(
            derive(
                DocTypeKey::PlanReviews,
                "2026-03-28-review-pr-migration-review-1.md",
                &NUMERIC
            )
            .as_deref(),
            Some("review-pr-migration")
        );
        assert_eq!(
            derive(
                DocTypeKey::PlanReviews,
                "2026-04-18-no-suffix.md",
                &NUMERIC
            ),
            None
        );
    }

    #[test]
    fn the_review_suffix_rule_holds_at_its_edges() {
        // Only the *trailing* -review-N is a suffix; an internal one is part of
        // the slug and survives.
        assert_eq!(
            derive(
                DocTypeKey::PlanReviews,
                "2026-03-28-review-pr-review-2-review-1.md",
                &NUMERIC
            )
            .as_deref(),
            Some("review-pr-review-2")
        );

        // A non-numeric tail is not a review suffix at all, so there is no
        // review to name and the slug is absent rather than silently truncated.
        assert_eq!(
            derive(
                DocTypeKey::PlanReviews,
                "2026-03-28-plan-review-final.md",
                &NUMERIC
            ),
            None
        );
    }

    #[test]
    fn work_item_reviews_accept_the_no_date_id_prefixed_shape() {
        assert_eq!(
            derive(
                DocTypeKey::WorkItemReviews,
                "0030-centralise-path-defaults-review-1.md",
                &NUMERIC
            )
            .as_deref(),
            Some("centralise-path-defaults")
        );
    }

    #[test]
    fn templates_have_no_slug() {
        assert_eq!(derive(DocTypeKey::Templates, "plan.md", &NUMERIC), None);
    }

    #[test]
    fn topic_research_takes_the_subject_directory_name_verbatim() {
        assert_eq!(
            derive(
                DocTypeKey::TopicResearch,
                "model-context-protocol.md",
                &NUMERIC
            )
            .as_deref(),
            Some("model-context-protocol")
        );
    }

    #[test]
    fn topic_research_guards_the_empty_stem() {
        assert_eq!(derive(DocTypeKey::TopicResearch, ".md", &NUMERIC), None);
    }

    #[test]
    fn humanise_slug_covers_prefix_and_casing_cases() {
        let cases = [
            ("design-token-system", "Design Token System"),
            ("0042-templates-review-1", "Templates Review 1"),
            ("2026-05-21-current-app-vs-claude", "Current App Vs Claude"),
            ("2026-05-21-0042-foo", "0042 Foo"),
            ("2026-05-21", "2026 05 21"),
            ("2026-05-21-", "2026 05 21"),
            ("foo--bar", "Foo Bar"),
            ("", ""),
        ];
        for (input, expected) in cases {
            assert_eq!(humanise_slug(input), expected, "{input}");
        }
    }
}
