//! Pins the compiled scan regex's own behaviour end to end.

mod common;

use common::TestError;

#[test]
#[allow(clippy::literal_string_with_formatting_args)]
fn the_compiled_scan_regex_drives_slug_and_id_extraction(
) -> Result<(), TestError> {
    use corpus::{DocTypeKey, WorkItemIdScheme};
    use corpus_adapters::compile_scan_regex;
    use corpus_adapters::RegexScanner;

    let scanner =
        RegexScanner::compile(&compile_scan_regex("{number:04d}", "")?)?;
    let scheme = WorkItemIdScheme::numeric();
    assert_eq!(
        corpus::slug::derive(
            DocTypeKey::WorkItems,
            "0001-three-layer-review.md",
            &scheme,
            &scanner
        )
        .as_deref(),
        Some("three-layer-review")
    );
    assert_eq!(
        scheme.extract_id("0042-foo.md", &scanner).as_deref(),
        Some("0042")
    );

    let scanner = RegexScanner::compile(&compile_scan_regex(
        "{project}-{number:04d}",
        "PROJ",
    )?)?;
    let scheme = WorkItemIdScheme {
        id_pattern: "{project}-{number:04d}".to_owned(),
        key: Some("PROJ".to_owned()),
    };
    assert_eq!(
        corpus::slug::derive(
            DocTypeKey::WorkItems,
            "PROJ-0042-ship-it.md",
            &scheme,
            &scanner
        )
        .as_deref(),
        Some("ship-it")
    );
    // A legacy bare-numeric file the project scan regex rejects still yields a
    // slug via the pure fallback.
    assert_eq!(
        corpus::slug::derive(
            DocTypeKey::WorkItems,
            "0042-legacy.md",
            &scheme,
            &scanner
        )
        .as_deref(),
        Some("legacy")
    );
    Ok(())
}
