//! The mark a baseline entry carries while the tracker holds a push it
//! stored with a different meaning, and the rule that sets and clears it.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Map;
use serde_json::Value;
use tracker::ExternalId;
use work::sync::BodyAgreement;

use crate::sync::push_fidelity::sections_of;
use crate::sync::push_fidelity::ChangedLine;
use crate::sync::push_fidelity::PushFidelity;
use crate::sync::push_fidelity::RewriteKind;
use crate::sync::push_fidelity::RewriteNotice;
use crate::sync::push_fidelity::RewrittenLines;

/// The rewrite a push left on the tracker, as persisted with the entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteMark {
    sections: Vec<String>,
    lines: Vec<MarkedLine>,
    more: usize,
    extra: BTreeMap<String, Value>,
}

/// One persisted changed line, with the fields a later build added to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkedLine {
    line: ChangedLine,
    extra: BTreeMap<String, Value>,
}

/// What an entry's stored `rewritten_on_push` holds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MarkState {
    #[default]
    Absent,
    Marked(Box<RewriteMark>),
    /// A value this build cannot read, carried only by a write that keeps
    /// the mark.
    Unreadable(Value),
}

/// How a baseline write treats the mark the entry already holds.
#[derive(Debug, Clone, Copy)]
pub enum MarkChange<'a> {
    /// A push's own read-back judged the body sent.
    Judged(&'a PushFidelity),
    /// A create judged its new issue; no earlier issue's mark carries over.
    Fresh(&'a PushFidelity),
    Clear,
    Keep,
}

impl MarkChange<'_> {
    /// How a write that reconciles without pushing a body treats the mark:
    /// cleared once classification confirmed the bodies agree, else kept.
    #[must_use]
    pub const fn after_reconciliation(agreement: BodyAgreement) -> Self {
        match agreement {
            BodyAgreement::Agreed => Self::Clear,
            BodyAgreement::Unconfirmed => Self::Keep,
        }
    }
}

impl RewriteMark {
    pub(crate) const KEY: &'static str = "rewritten_on_push";
    const FIELDS: [&'static str; 3] = ["lines", "more", "sections"];

    /// The mark a push's verdict leaves over `previous`.
    #[must_use]
    pub fn after(
        fidelity: &PushFidelity,
        previous: Option<&Self>,
    ) -> Option<Self> {
        match fidelity {
            PushFidelity::Rewritten { changes } => Some(Self::from(
                RewrittenLines::bounded(changes.as_slice().to_vec()),
            )),
            PushFidelity::Preserved => None,
            PushFidelity::Unjudged => previous.cloned(),
        }
    }

    /// The notice of the rewrite this mark records, standing on the item at
    /// `path` known remotely as `external_id`.
    #[must_use]
    pub fn notice(
        &self,
        path: &Path,
        external_id: &ExternalId,
    ) -> RewriteNotice {
        RewriteNotice {
            kind: RewriteKind::StillOnPush,
            path: path.to_path_buf(),
            external_id: external_id.clone(),
            lines: self.rewritten_lines(),
        }
    }

    /// The lines a notice of this mark shows.
    #[must_use]
    pub fn rewritten_lines(&self) -> RewrittenLines {
        RewrittenLines {
            sections: self.sections.clone(),
            lines: Self::changed_lines(&self.lines),
            more: self.more,
        }
    }

    fn parse(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        let lines = object
            .get("lines")?
            .as_array()?
            .iter()
            .map(MarkedLine::parse)
            .collect::<Option<Vec<_>>>()?;
        let more = object
            .get("more")
            .and_then(Value::as_u64)
            .and_then(|more| usize::try_from(more).ok())
            .unwrap_or(0);
        let sections = object.get("sections").map_or_else(
            || Some(sections_of(&Self::changed_lines(&lines))),
            |sections| {
                sections
                    .as_array()?
                    .iter()
                    .map(|section| section.as_str().map(str::to_owned))
                    .collect()
            },
        )?;
        Some(Self {
            sections,
            lines,
            more,
            extra: extra_fields(object, &Self::FIELDS),
        })
    }

    fn changed_lines(lines: &[MarkedLine]) -> Vec<ChangedLine> {
        lines.iter().map(|marked| marked.line.clone()).collect()
    }

    fn render(&self) -> String {
        let mut fields = rendered_extra(&self.extra);
        let lines: Vec<String> =
            self.lines.iter().map(MarkedLine::render).collect();
        fields.insert("lines".to_owned(), format!("[{}]", lines.join(",")));
        fields.insert("more".to_owned(), self.more.to_string());
        let sections: Vec<String> =
            self.sections.iter().map(|s| render_string(s)).collect();
        fields
            .insert("sections".to_owned(), format!("[{}]", sections.join(",")));
        render_object(&fields)
    }
}

impl From<RewrittenLines> for RewriteMark {
    fn from(rewritten: RewrittenLines) -> Self {
        Self {
            sections: rewritten.sections,
            lines: rewritten
                .lines
                .into_iter()
                .map(|line| MarkedLine {
                    line,
                    extra: BTreeMap::new(),
                })
                .collect(),
            more: rewritten.more,
            extra: BTreeMap::new(),
        }
    }
}

impl MarkedLine {
    const FIELDS: [&'static str; 3] = ["section", "sent", "stored"];

    fn parse(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        let text = |key: &str| object.get(key)?.as_str().map(str::to_owned);
        Some(Self {
            line: ChangedLine {
                section: text("section")?,
                sent: text("sent")?,
                stored: text("stored")?,
            },
            extra: extra_fields(object, &Self::FIELDS),
        })
    }

    fn render(&self) -> String {
        let mut fields = rendered_extra(&self.extra);
        for (key, text) in Self::FIELDS.into_iter().zip([
            &self.line.section,
            &self.line.sent,
            &self.line.stored,
        ]) {
            fields.insert(key.to_owned(), render_string(text));
        }
        render_object(&fields)
    }
}

impl MarkState {
    pub(crate) fn parse(object: &Map<String, Value>) -> Self {
        object.get(RewriteMark::KEY).map_or(Self::Absent, |value| {
            RewriteMark::parse(value).map_or_else(
                || Self::Unreadable(value.clone()),
                |mark| Self::Marked(Box::new(mark)),
            )
        })
    }

    /// The value persisted under [`RewriteMark::KEY`], if any.
    pub(crate) fn rendered(&self) -> Option<String> {
        match self {
            Self::Absent => None,
            Self::Marked(mark) => Some(mark.render()),
            Self::Unreadable(raw) => Some(raw.to_string()),
        }
    }

    #[must_use]
    pub const fn mark(&self) -> Option<&RewriteMark> {
        match self {
            Self::Marked(mark) => Some(&**mark),
            Self::Absent | Self::Unreadable(_) => None,
        }
    }

    /// The state `change` leaves over this one.
    #[must_use]
    pub fn after(&self, change: MarkChange<'_>) -> Self {
        let marked = |mark: Option<RewriteMark>| {
            mark.map_or(Self::Absent, |mark| Self::Marked(Box::new(mark)))
        };
        match change {
            MarkChange::Judged(PushFidelity::Unjudged) | MarkChange::Keep => {
                self.clone()
            }
            MarkChange::Judged(fidelity) => {
                marked(RewriteMark::after(fidelity, self.mark()))
            }
            MarkChange::Fresh(fidelity) => {
                marked(RewriteMark::after(fidelity, None))
            }
            MarkChange::Clear => Self::Absent,
        }
    }
}

fn extra_fields(
    object: &Map<String, Value>,
    known: &[&str],
) -> BTreeMap<String, Value> {
    object
        .iter()
        .filter(|(key, _)| !known.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn rendered_extra(extra: &BTreeMap<String, Value>) -> BTreeMap<String, String> {
    extra
        .iter()
        .map(|(key, value)| (key.clone(), value.to_string()))
        .collect()
}

pub(crate) fn render_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// A JSON object from already-rendered values, its keys in the map's
/// alphabetical order.
pub(crate) fn render_object(fields: &BTreeMap<String, String>) -> String {
    let members: Vec<String> = fields
        .iter()
        .map(|(key, value)| format!("{}:{value}", render_string(key)))
        .collect();
    format!("{{{}}}", members.join(","))
}

#[cfg(test)]
mod tests {
    use work::sync::BodyAgreement;

    use super::MarkChange;
    use super::MarkState;
    use super::RewriteMark;
    use crate::sync::push_fidelity::ChangedLine;
    use crate::sync::push_fidelity::PushFidelity;
    use crate::sync::push_fidelity::RewrittenLines;

    fn change(section: &str) -> ChangedLine {
        ChangedLine {
            section: section.to_owned(),
            sent: "See crates.io.".to_owned(),
            stored: "See [crates.io](<http://crates.io>).".to_owned(),
        }
    }

    fn rewritten(section: &str) -> PushFidelity {
        PushFidelity::Rewritten {
            changes: change(section).into(),
        }
    }

    fn mark(section: &str) -> RewriteMark {
        RewriteMark::from(RewrittenLines::bounded(vec![change(section)]))
    }

    #[test]
    fn rewrite_mark_after_follows_the_verdict() {
        let previous = mark("Earlier");

        assert_eq!(
            RewriteMark::after(&rewritten("Notes"), Some(&previous)),
            Some(mark("Notes"))
        );
        assert_eq!(
            RewriteMark::after(&PushFidelity::Preserved, Some(&previous)),
            None
        );
        assert_eq!(
            RewriteMark::after(&PushFidelity::Unjudged, Some(&previous)),
            Some(previous.clone())
        );
        assert_eq!(RewriteMark::after(&PushFidelity::Unjudged, None), None);
    }

    #[test]
    fn rewrite_mark_after_a_fresh_create_ignores_the_previous_mark() {
        let previous = MarkState::Marked(Box::new(mark("Earlier")));

        assert_eq!(
            previous.after(MarkChange::Fresh(&PushFidelity::Unjudged)),
            MarkState::Absent
        );
        assert_eq!(
            previous.after(MarkChange::Judged(&PushFidelity::Unjudged)),
            previous
        );
    }

    #[test]
    fn a_reconciliation_clears_the_mark_only_when_the_bodies_agreed() {
        let marked = MarkState::Marked(Box::new(mark("Notes")));

        assert_eq!(
            marked
                .after(MarkChange::after_reconciliation(BodyAgreement::Agreed)),
            MarkState::Absent
        );
        assert_eq!(
            marked.after(MarkChange::after_reconciliation(
                BodyAgreement::Unconfirmed
            )),
            marked
        );
    }

    fn persisted(state: &MarkState) -> serde_json::Result<MarkState> {
        let mut object = serde_json::Map::new();
        if let Some(rendered) = state.rendered() {
            object.insert(
                RewriteMark::KEY.to_owned(),
                serde_json::from_str(&rendered)?,
            );
        }
        Ok(MarkState::parse(&object))
    }

    fn sections(state: &MarkState) -> Option<Vec<String>> {
        state.mark().map(|mark| mark.rewritten_lines().sections)
    }

    #[test]
    fn a_persisted_mark_names_every_section_including_unshown_lines(
    ) -> serde_json::Result<()> {
        let mut changes = vec![change("Notes"); 5];
        changes.push(change("Summary"));
        let state = MarkState::Marked(Box::new(RewriteMark::from(
            RewrittenLines::bounded(changes),
        )));

        let reread = persisted(&state)?;

        assert_eq!(
            sections(&reread),
            Some(vec!["Notes".to_owned(), "Summary".to_owned()])
        );
        Ok(())
    }

    #[test]
    fn a_mark_persisted_without_sections_names_those_of_its_lines() {
        let line = serde_json::json!({
            "section": "Notes",
            "sent": "a",
            "stored": "b",
        });
        let object = serde_json::Map::from_iter([(
            RewriteMark::KEY.to_owned(),
            serde_json::json!({ "lines": [line], "more": 3 }),
        )]);

        let state = MarkState::parse(&object);

        assert_eq!(sections(&state), Some(vec!["Notes".to_owned()]));
    }

    #[test]
    fn an_unreadable_mark_survives_every_change_without_a_verdict() {
        let unreadable = MarkState::Unreadable(serde_json::json!("garbled"));

        assert_eq!(unreadable.after(MarkChange::Keep), unreadable);
        assert_eq!(
            unreadable.after(MarkChange::Judged(&PushFidelity::Unjudged)),
            unreadable
        );
        assert_eq!(
            unreadable.after(MarkChange::Judged(&PushFidelity::Preserved)),
            MarkState::Absent
        );
        assert_eq!(unreadable.after(MarkChange::Clear), MarkState::Absent);
    }
}
