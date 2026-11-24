//! The tab-separated records a command prints to stdout, each field made
//! safe so no value can split or forge a record.

use std::fmt::Display;
use std::fmt::Formatter;

use work_adapters::sync::push_fidelity::RewriteKind;
use work_adapters::sync::push_fidelity::RewriteNotice;

#[derive(Debug, Default)]
pub struct TsvRecord {
    fields: Vec<String>,
}

impl TsvRecord {
    #[must_use]
    pub fn of<I, F>(fields: I) -> Self
    where
        I: IntoIterator<Item = F>,
        F: Display,
    {
        Self {
            fields: fields
                .into_iter()
                .map(|field| printable(&single_line(&field.to_string())))
                .collect(),
        }
    }
}

impl Display for TsvRecord {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.fields.join("\t"))
    }
}

/// Collapses tab, newline and carriage return to a space, so a value cannot
/// break the single-record TSV format.
pub fn single_line(text: &str) -> String {
    text.replace(['\t', '\n', '\r'], " ")
}

/// Escapes every control character but the newline, so text from paths,
/// filenames or a tracker cannot move the cursor, clear the screen or
/// overwrite a line, while a deliberately multi-line warning keeps its
/// layout.
pub fn printable(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() && character != '\n' {
                character.escape_default().collect()
            } else {
                character.to_string()
            }
        })
        .collect()
}

/// A failure's stderr line, built from its parts so no cause can forge a
/// second line; `failed` is `failed`, or `would fail` on a preview.
#[must_use]
pub fn failure_line(
    failure: &work_adapters::sync::report::RunFailure,
    failed: &str,
) -> String {
    let label = match failure.severity {
        work_adapters::sync::report::FailureSeverity::Error => "error",
        work_adapters::sync::report::FailureSeverity::Warning => "warning",
    };
    format!(
        "{label}: {}: {} {failed}: {}",
        single_line(&failure.subject),
        failure.operation,
        single_line(&failure.cause)
    )
}

/// Where a person reports a tracker storing a push with a different meaning.
const ISSUES: &str = "https://github.com/atomicinnovation/accelerator/issues";

/// The stderr block for one push `tracker` stored with a different
/// meaning, each line naming `label`; every value is reduced to one line,
/// since the stored text comes from the tracker, and the sent and stored
/// text keep each line boundary as a [`LINE_BOUNDARY`].
#[must_use]
pub fn rewrite_lines(
    notice: &RewriteNotice,
    label: &str,
    tracker: &str,
) -> Vec<String> {
    let shown = |text: &str| printable(&single_line(text));
    let shown_lines = |text: &str| shown(&with_line_boundaries_marked(text));
    let label = shown(label);
    let note = |text: String| format!("note: {label}: {text}");
    let rewritten = &notice.lines;
    let standing = notice.kind == RewriteKind::StillOnPush;
    let mut lines = vec![format!(
        "warning: {label}: {}rewritten on push: {}",
        if standing { "still " } else { "" },
        shown(&rewritten.sections.join(", "))
    )];
    lines.extend(rewritten.lines.iter().map(|line| {
        note(format!(
            "{}: sent `{}`, stored `{}`",
            shown(&line.section),
            shown_lines(&line.sent),
            shown_lines(&line.stored)
        ))
    }));
    if rewritten.more > 0 {
        lines.push(note(format!(
            "… and {} more changed lines; `accelerator {} show {}` shows the \
             stored description",
            rewritten.more,
            shown(tracker),
            shown(notice.external_id.as_str())
        )));
    }
    if !standing {
        lines.push(note(format!(
            "the local file is unchanged, but {} stored these lines with a \
             different meaning",
            shown(&tracker_name(tracker))
        )));
    }
    lines.push(note(format!(
        "to keep the meaning, put the sent text in backticks or reword it, \
         then run `accelerator work update {} --push`",
        shown(&notice.path.display().to_string())
    )));
    if !standing {
        lines.push(note(format!(
            "please report it at {ISSUES} with the lines above"
        )));
    }
    lines
}

/// How a notice shows a line break in sent or stored text. Not `\n`,
/// which a reader could not tell from a backslash escape the text holds.
const LINE_BOUNDARY: char = '⏎';

fn with_line_boundaries_marked(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace(['\n', '\r'], &LINE_BOUNDARY.to_string())
}

/// As [`rewrite_lines`], naming the item's file relative to `repo_root`.
#[must_use]
pub fn rewrite_lines_under(
    repo_root: &std::path::Path,
    notice: &RewriteNotice,
    label: &str,
    tracker: &str,
) -> Vec<String> {
    let relative = RewriteNotice {
        path: notice
            .path
            .strip_prefix(repo_root)
            .unwrap_or(&notice.path)
            .to_path_buf(),
        ..notice.clone()
    };
    rewrite_lines(&relative, label, tracker)
}

/// `linear` as a person names it: `Linear`.
fn tracker_name(integration: &str) -> String {
    let mut characters = integration.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(characters).collect()
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tracker::ExternalId;
    use work_adapters::sync::push_fidelity::ChangedLine;
    use work_adapters::sync::push_fidelity::RewriteKind;
    use work_adapters::sync::push_fidelity::RewriteNotice;
    use work_adapters::sync::push_fidelity::RewrittenLines;

    use super::rewrite_lines;
    use super::TsvRecord;

    fn notice(changes: Vec<ChangedLine>) -> RewriteNotice {
        RewriteNotice {
            kind: RewriteKind::OnPush,
            path: Path::new("meta/work/0001-search.md").to_path_buf(),
            external_id: ExternalId::new("ENG-7".to_owned()),
            lines: RewrittenLines::bounded(changes),
        }
    }

    fn still(changes: Vec<ChangedLine>) -> RewriteNotice {
        RewriteNotice {
            kind: RewriteKind::StillOnPush,
            ..notice(changes)
        }
    }

    fn changed(section: &str, sent: &str, stored: &str) -> ChangedLine {
        ChangedLine {
            section: section.to_owned(),
            sent: sent.to_owned(),
            stored: stored.to_owned(),
        }
    }

    fn numbered(count: usize) -> Vec<ChangedLine> {
        (0..count)
            .map(|index| changed("Notes", &format!("s{index}"), "x"))
            .collect()
    }

    #[test]
    fn a_rewrite_renders_its_warning_and_notes() {
        let lines = rewrite_lines(
            &notice(vec![changed(
                "Notes",
                "See crates.io.",
                "See [crates.io](<http://crates.io>).",
            )]),
            "0001",
            "linear",
        );

        assert_eq!(
            lines,
            vec![
                "warning: 0001: rewritten on push: Notes",
                "note: 0001: Notes: sent `See crates.io.`, stored `See \
                 [crates.io](<http://crates.io>).`",
                "note: 0001: the local file is unchanged, but Linear stored \
                 these lines with a different meaning",
                "note: 0001: to keep the meaning, put the sent text in \
                 backticks or reword it, then run `accelerator work update \
                 meta/work/0001-search.md --push`",
                "note: 0001: please report it at \
                 https://github.com/atomicinnovation/accelerator/issues with \
                 the lines above",
            ]
        );
    }

    #[test]
    fn a_standing_rewrite_renders_its_lines_and_the_clearing_note_alone() {
        let lines = rewrite_lines(
            &still(vec![changed(
                "Notes",
                "See crates.io.",
                "See [crates.io](<http://crates.io>).",
            )]),
            "0001",
            "linear",
        );

        assert_eq!(
            lines,
            vec![
                "warning: 0001: still rewritten on push: Notes",
                "note: 0001: Notes: sent `See crates.io.`, stored `See \
                 [crates.io](<http://crates.io>).`",
                "note: 0001: to keep the meaning, put the sent text in \
                 backticks or reword it, then run `accelerator work update \
                 meta/work/0001-search.md --push`",
            ]
        );
    }

    #[test]
    fn a_standing_rewrite_names_its_overflow() {
        let lines = rewrite_lines(&still(numbered(7)), "0001", "jira");

        assert!(lines.contains(
            &"note: 0001: … and 2 more changed lines; `accelerator jira show \
              ENG-7` shows the stored description"
                .to_owned()
        ));
    }

    #[test]
    fn merged_paragraphs_show_the_boundary_they_lost() {
        for notice in [notice, still] {
            let lines = rewrite_lines(
                &notice(vec![changed("Notes", "a\n\nb", "a\r\nb")]),
                "0001",
                "linear",
            );

            assert_eq!(
                lines[1],
                "note: 0001: Notes: sent `a⏎⏎b`, stored `a⏎b`"
            );
        }
    }

    #[test]
    fn a_multi_line_change_renders_on_one_line_marking_its_boundaries() {
        let lines = rewrite_lines(
            &notice(vec![changed(
                "X09",
                "- owned by\n  0160.\n- next",
                "* owned by\n  160. \n* next",
            )]),
            "0001",
            "linear",
        );

        let shown: Vec<&String> = lines
            .iter()
            .filter(|line| line.contains("sent `"))
            .collect();
        assert_eq!(
            shown,
            ["note: 0001: X09: sent `- owned by⏎  0160.⏎- next`, stored \
              `* owned by⏎  160. ⏎* next`"]
        );
        assert!(lines
            .iter()
            .all(|line| line.starts_with("warning: ")
                || line.starts_with("note: ")));
    }

    #[test]
    fn the_overflow_note_names_the_external_id() {
        let lines = rewrite_lines(&notice(numbered(6)), "0001", "jira");

        assert!(lines.contains(
            &"note: 0001: … and 1 more changed lines; `accelerator jira show \
              ENG-7` shows the stored description"
                .to_owned()
        ));
    }

    #[test]
    fn changed_lines_are_capped_at_five() {
        for (changes, overflow) in [
            (5, None),
            (6, Some("… and 1 more changed lines")),
            (7, Some("… and 2 more changed lines")),
        ] {
            let lines =
                rewrite_lines(&notice(numbered(changes)), "0001", "linear");
            let shown = lines
                .iter()
                .filter(|line| line.contains(": sent `"))
                .count();
            assert_eq!(shown, 5, "{changes}");
            let overflowed: Vec<&String> = lines
                .iter()
                .filter(|line| line.contains("more changed"))
                .collect();
            match overflow {
                None => assert!(overflowed.is_empty(), "{changes}"),
                Some(text) => {
                    assert_eq!(overflowed.len(), 1, "{changes}");
                    assert!(overflowed[0].contains(text), "{overflowed:?}");
                }
            }
        }
    }

    #[test]
    fn a_stored_line_cannot_forge_another_line() {
        let lines = rewrite_lines(
            &notice(vec![changed("Notes", "a", "b\r\nwarning: forged\t")]),
            "0001",
            "linear",
        );

        assert!(lines.iter().all(|line| !line.contains('\n')));
        assert!(lines
            .iter()
            .all(|line| !line.starts_with("warning: forged")));
    }

    #[test]
    fn a_filename_with_a_newline_and_tab_yields_one_record() {
        let record =
            TsvRecord::of(["#", "kept-edits", "0001", "a\nb\tc\x1bd.md"]);

        assert_eq!(record.to_string(), "#\tkept-edits\t0001\ta b c\\u{1b}d.md");
    }
}
