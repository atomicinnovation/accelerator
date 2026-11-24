//! Whether a tracker stored a pushed body with the meaning it was sent
//! with, judged from the read-back that follows the push.

use std::ops::Range;
use std::path::Path;
use std::path::PathBuf;

use similar::DiffOp;
use similar::TextDiff;
use tracker::ExternalId;
use work::section_diff::frontmatter_and_body;
use work::section_diff::SectionDiff;
use work::sync::BodyEquivalence;

use crate::sync::body_comparison::BodyComparison;

/// What a push's read-back says about the body sent.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PushFidelity {
    Preserved,
    Rewritten {
        changes: ChangedLines,
    },
    /// Nothing could be judged: the remote was not read, has no Markdown
    /// form, or the tracker refuses the body sent.
    #[default]
    Unjudged,
}

/// Lines of one section that the tracker stored with a different meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedLine {
    pub section: String,
    pub sent: String,
    pub stored: String,
}

/// The lines a rewrite changed: never none, since every rewrite lies
/// somewhere in the body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedLines(Vec<ChangedLine>);

impl ChangedLines {
    /// `lines`, unless there are none.
    #[must_use]
    pub fn of(lines: Vec<ChangedLine>) -> Option<Self> {
        (!lines.is_empty()).then_some(Self(lines))
    }

    /// The whole body as sent and as stored, for a rewrite that no one
    /// section holds, such as sections stored in another order.
    #[must_use]
    pub fn across_whole_body(sent: &str, stored: &str) -> Self {
        let whole = |body: &str| body.trim_matches(['\n', '\r']).to_owned();
        Self(vec![ChangedLine {
            section: "(whole body)".to_owned(),
            sent: whole(sent),
            stored: whole(stored),
        }])
    }

    #[must_use]
    pub fn as_slice(&self) -> &[ChangedLine] {
        &self.0
    }

    #[must_use]
    pub fn into_vec(self) -> Vec<ChangedLine> {
        self.0
    }
}

impl From<ChangedLine> for ChangedLines {
    fn from(line: ChangedLine) -> Self {
        Self(vec![line])
    }
}

/// How many changed lines a notice shows before counting the rest.
pub const SHOWN_LINES: usize = 5;

/// The most bytes a notice shows of either side of a changed line.
pub const LINE_BYTES: usize = 200;

/// A push the tracker stored with a different meaning, for a person to
/// act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteNotice {
    pub kind: RewriteKind,
    pub path: PathBuf,
    pub external_id: ExternalId,
    pub lines: RewrittenLines,
}

/// Whether a notice is of the push this run made, or of an earlier push
/// whose rewrite the tracker still holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewriteKind {
    OnPush,
    StillOnPush,
}

impl RewriteNotice {
    /// The notice a push's verdict calls for: one only when it was
    /// rewritten.
    #[must_use]
    pub fn on_push(
        fidelity: PushFidelity,
        path: &Path,
        external_id: &ExternalId,
    ) -> Option<Self> {
        let PushFidelity::Rewritten { changes } = fidelity else {
            return None;
        };
        Some(Self {
            kind: RewriteKind::OnPush,
            path: path.to_path_buf(),
            external_id: external_id.clone(),
            lines: RewrittenLines::bounded(changes.into_vec()),
        })
    }
}

/// The changed lines a notice shows, so every rendering of one rewrite
/// shows the same text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewrittenLines {
    /// Every section a changed line falls in, shown or not.
    pub sections: Vec<String>,
    pub lines: Vec<ChangedLine>,
    /// The changed lines left unshown.
    pub more: usize,
}

impl RewrittenLines {
    #[must_use]
    pub fn bounded(changes: Vec<ChangedLine>) -> Self {
        let sections = sections_of(&changes);
        let more = changes.len().saturating_sub(SHOWN_LINES);
        let lines = changes
            .into_iter()
            .take(SHOWN_LINES)
            .map(|change| ChangedLine {
                sent: cut(&change.sent),
                stored: cut(&change.stored),
                section: change.section,
            })
            .collect();
        Self {
            sections,
            lines,
            more,
        }
    }
}

/// The sections `lines` fall in, each once, in order of first appearance.
pub(crate) fn sections_of(lines: &[ChangedLine]) -> Vec<String> {
    let mut sections: Vec<String> = Vec::new();
    for line in lines {
        if !sections.contains(&line.section) {
            sections.push(line.section.clone());
        }
    }
    sections
}

fn cut(side: &str) -> String {
    if side.len() <= LINE_BYTES {
        return side.to_owned();
    }
    let boundary = (0..=LINE_BYTES)
        .rev()
        .find(|&index| side.is_char_boundary(index))
        .unwrap_or_default();
    format!("{}…", &side[..boundary])
}

impl BodyComparison<'_> {
    /// Whether the `remote_projection` read back after pushing
    /// `local_content` means what was sent, and if not, where it differs.
    #[must_use]
    pub fn push_fidelity(
        &self,
        local_content: &str,
        remote_projection: Option<&str>,
    ) -> PushFidelity {
        let Some(projection) = remote_projection else {
            return PushFidelity::Unjudged;
        };
        let (_, local_body) = frontmatter_and_body(local_content);
        if self.rejects(&local_body) {
            return PushFidelity::Unjudged;
        }
        let Ok(stored_body) = self.local_body(projection) else {
            return PushFidelity::Unjudged;
        };
        if self.equivalence_to_rendered(&local_body, &stored_body)
            == BodyEquivalence::Equivalent
        {
            return PushFidelity::Preserved;
        }
        let changes = ChangedLines::of(
            self.differing_sections_as_written(&local_body, &stored_body)
                .iter()
                .flat_map(|section| changed_lines(section, self))
                .collect(),
        )
        .unwrap_or_else(|| {
            ChangedLines::across_whole_body(&local_body, &stored_body)
        });
        PushFidelity::Rewritten { changes }
    }
}

/// The lines of `section` the tracker changed the meaning of, each hunk
/// judged in its section; every changed hunk when none alone does.
#[must_use]
pub fn changed_lines(
    section: &SectionDiff,
    comparison: &BodyComparison<'_>,
) -> Vec<ChangedLine> {
    let meaningful = meaningful_hunks(section, comparison);
    if meaningful.is_empty() {
        SectionLines::of(section)
            .hunks()
            .map(|hunk| hunk.changed_line(section))
            .collect()
    } else {
        meaningful
    }
}

/// The hunks of `section` whose stored lines, spliced in place of the
/// sent ones, change how the section reads.
#[must_use]
pub fn meaningful_hunks(
    section: &SectionDiff,
    comparison: &BodyComparison<'_>,
) -> Vec<ChangedLine> {
    let lines = SectionLines::of(section);
    lines
        .hunks()
        .filter(|hunk| {
            !comparison.reads_alike(&section.local, &lines.spliced(hunk))
        })
        .map(|hunk| hunk.changed_line(section))
        .collect()
}

struct SectionLines<'a> {
    sent: Vec<&'a str>,
    stored: Vec<&'a str>,
    ops: Vec<DiffOp>,
}

impl<'a> SectionLines<'a> {
    fn of(section: &'a SectionDiff) -> Self {
        let diff = TextDiff::from_lines(&section.local, &section.remote);
        Self {
            sent: diff.old_slices().to_vec(),
            stored: diff.new_slices().to_vec(),
            ops: diff.ops().to_vec(),
        }
    }

    /// Each run of adjacent changes, however many lines each side holds,
    /// widened over the unchanged lines around it while neither side holds
    /// any text, so a change of line boundaries shows the lines it joins
    /// or splits.
    fn hunks(&self) -> impl Iterator<Item = Hunk> + '_ {
        let mut hunks: Vec<(Range<usize>, Range<usize>)> = Vec::new();
        let mut open = false;
        for op in &self.ops {
            if matches!(op, DiffOp::Equal { .. }) {
                open = false;
                continue;
            }
            match hunks.last_mut() {
                Some((sent, stored)) if open => {
                    sent.end = op.old_range().end;
                    stored.end = op.new_range().end;
                }
                _ => hunks.push((op.old_range(), op.new_range())),
            }
            open = true;
        }
        hunks.into_iter().map(|(sent, stored)| {
            let (sent, stored) = self.showing_text(sent, stored);
            Hunk {
                sent: self.sent[sent.clone()].concat(),
                stored: self.stored[stored].concat(),
                sent_lines: sent,
            }
        })
    }

    fn showing_text(
        &self,
        mut sent: Range<usize>,
        mut stored: Range<usize>,
    ) -> (Range<usize>, Range<usize>) {
        let is_blank =
            |lines: &[&str]| lines.iter().all(|line| line.trim().is_empty());
        while is_blank(&self.sent[sent.clone()])
            && is_blank(&self.stored[stored.clone()])
        {
            let before = sent.start > 0 && stored.start > 0;
            let after =
                sent.end < self.sent.len() && stored.end < self.stored.len();
            if !before && !after {
                break;
            }
            if before {
                sent.start -= 1;
                stored.start -= 1;
            }
            if after {
                sent.end += 1;
                stored.end += 1;
            }
        }
        (sent, stored)
    }

    /// The sent section with `hunk`'s stored lines in place of its sent
    /// ones.
    fn spliced(&self, hunk: &Hunk) -> String {
        let mut spliced = self.sent[..hunk.sent_lines.start].concat();
        spliced.push_str(&hunk.stored);
        spliced.push_str(&self.sent[hunk.sent_lines.end..].concat());
        spliced
    }
}

struct Hunk {
    sent: String,
    stored: String,
    sent_lines: Range<usize>,
}

impl Hunk {
    fn changed_line(&self, section: &SectionDiff) -> ChangedLine {
        ChangedLine {
            section: section.name.clone(),
            sent: without_line_ending(&self.sent),
            stored: without_line_ending(&self.stored),
        }
    }
}

fn without_line_ending(lines: &str) -> String {
    lines.trim_end_matches(['\n', '\r']).to_owned()
}
