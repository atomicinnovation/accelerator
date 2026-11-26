use std::num::NonZeroU32;

use corpus::set::MemberKind;
use corpus::set::SetMatcher;
use corpus::set::SetPlace;
use research::topic::layout::lineage::Lineage;
use research::topic::layout::note_ref::NoteRef;
use research::topic::layout::slug::ReportSlug;
use research::topic::layout::slug::TakenNames;
use research::topic::layout::stem::Stem;
use research::topic::layout::BRIEF;
use research::topic::layout::OUTLINE;
use research::topic::layout::SYNTHESIS;
use research_declarations::topic::SET;

fn every_built_path() -> Option<Vec<String>> {
    let finding = Stem::allocated(1, "cost-model", "web")?;
    let nested = Lineage::root()
        .child(NonZeroU32::MIN)
        .child(NonZeroU32::new(3)?);
    let report = ReportSlug::from("What does caching cost?");
    Some(vec![
        BRIEF.to_owned(),
        OUTLINE.to_owned(),
        SYNTHESIS.to_owned(),
        finding.finding_path(),
        Stem::allocated(12, "latency", "scholarly")?.finding_path(),
        NoteRef {
            stem: finding.clone(),
            lineage: Lineage::root(),
        }
        .path(),
        NoteRef {
            stem: finding,
            lineage: nested,
        }
        .path(),
        report.free_report_path(&TakenNames::default()),
        report.free_report_path(&TakenNames::listed(vec![
            "what-does-caching-cost.md".to_owned(),
        ])),
    ])
}

#[test]
fn every_path_a_topic_set_builds_is_a_document_member_of_its_shape(
) -> Result<(), String> {
    let set = SetMatcher::new(&SET).map_err(|error| format!("{error:?}"))?;
    let paths = every_built_path().ok_or("a path builder refused")?;
    assert_eq!(paths.len(), 9);
    for path in paths {
        assert_eq!(
            set.classify(&path),
            Some(SetPlace::Member(MemberKind::Document)),
            "{path}"
        );
    }
    Ok(())
}
