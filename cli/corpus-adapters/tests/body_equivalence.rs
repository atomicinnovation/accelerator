use std::path::{Path, PathBuf};

use corpus::body::{equivalent, BodyParser};
use corpus_adapters::CommonMarkBody;

type TestError = Box<dyn std::error::Error>;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../work-adapters/tests/fixtures")
}

struct Capture {
    pushed: String,
    read_back: String,
}

impl Capture {
    fn load() -> Result<Self, TestError> {
        let directory = fixtures().join("linear-rewrites");
        Ok(Self {
            pushed: std::fs::read_to_string(directory.join("pushed.md"))?,
            read_back: description(&directory.join("read-back.json"))?,
        })
    }

    fn fragments(&self, name: &str) -> Result<(String, String), TestError> {
        Ok((
            section(&self.pushed, name)?,
            section(&self.read_back, name)?,
        ))
    }

    fn without(&self, names: &[&str]) -> (String, String) {
        (
            without_sections(&self.pushed, names),
            without_sections(&self.read_back, names),
        )
    }
}

fn description(remote: &Path) -> Result<String, TestError> {
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(remote)?)?;
    json.pointer("/data/issue/description")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            format!("{} has no description", remote.display()).into()
        })
}

fn sections(markdown: &str) -> Vec<(&str, String)> {
    let mut sections: Vec<(&str, String)> = Vec::new();
    for line in markdown.lines() {
        if line.starts_with("## ") {
            sections.push((line, String::new()));
        } else if let Some((_, content)) = sections.last_mut() {
            content.push_str(line);
            content.push('\n');
        }
    }
    sections
}

fn names(heading: &str, name: &str) -> bool {
    heading
        .strip_prefix("## ")
        .and_then(|title| title.split(' ').next())
        == Some(name)
}

fn section(markdown: &str, name: &str) -> Result<String, TestError> {
    sections(markdown)
        .into_iter()
        .find(|(heading, _)| names(heading, name))
        .map(|(_, content)| content)
        .ok_or_else(|| format!("no section {name}").into())
}

fn without_sections(markdown: &str, excluded: &[&str]) -> String {
    sections(markdown)
        .into_iter()
        .filter(|(heading, _)| {
            !excluded.iter().any(|name| names(heading, name))
        })
        .fold(String::new(), |mut kept, (heading, content)| {
            kept.push_str(heading);
            kept.push('\n');
            kept.push_str(&content);
            kept
        })
}

fn equivalent_markdown(left: &str, right: &str) -> bool {
    equivalent(&CommonMarkBody.parse(left), &CommonMarkBody.parse(right))
}

fn captured_row_is_noise(name: &str) -> Result<(), TestError> {
    let (pushed, read_back) = Capture::load()?.fragments(name)?;
    assert_ne!(pushed.trim(), read_back.trim(), "{name} was not rewritten");
    assert!(
        equivalent_markdown(&pushed, &read_back),
        "{name}: {pushed:?} and {read_back:?} should be equivalent"
    );
    Ok(())
}

fn captured_row_differs(name: &str) -> Result<(), TestError> {
    let (pushed, read_back) = Capture::load()?.fragments(name)?;
    assert!(
        !equivalent_markdown(&pushed, &read_back),
        "{name}: {pushed:?} and {read_back:?} should differ"
    );
    Ok(())
}

fn assert_noise(left: &str, right: &str) {
    assert!(
        equivalent_markdown(left, right),
        "{left:?} and {right:?} should be equivalent"
    );
}

fn assert_differ(left: &str, right: &str) {
    assert!(
        !equivalent_markdown(left, right),
        "{left:?} and {right:?} should differ"
    );
}

#[test]
fn escaping_is_noise() -> Result<(), TestError> {
    captured_row_is_noise("R01")
}

#[test]
fn joined_code_span_lines_are_noise() -> Result<(), TestError> {
    let item = fixtures().join("work-item-sync-recovery/0293");
    let local = std::fs::read_to_string(item.join("local.md"))?;
    let remote = description(&item.join("remote.json"))?;
    let (local, remote) =
        (section(&local, "Summary")?, section(&remote, "Summary")?);
    assert_ne!(local, remote);
    assert_noise(&local, &remote);
    Ok(())
}

#[test]
fn per_line_bold_is_noise() -> Result<(), TestError> {
    captured_row_is_noise("R03")
}

#[test]
fn bold_hoisted_off_code_is_noise() -> Result<(), TestError> {
    captured_row_is_noise("R04")
}

#[test]
fn angle_bracketed_destinations_are_noise() -> Result<(), TestError> {
    captured_row_is_noise("R05")
}

#[test]
fn autolinked_urls_are_noise() -> Result<(), TestError> {
    captured_row_is_noise("R06")
}

#[test]
fn bullet_markers_are_noise() -> Result<(), TestError> {
    captured_row_is_noise("R07")
}

#[test]
fn task_marker_case_is_noise() -> Result<(), TestError> {
    captured_row_is_noise("R08")
}

#[test]
fn table_delimiter_rows_are_noise() -> Result<(), TestError> {
    captured_row_is_noise("R09")
}

#[test]
fn a_blank_line_before_a_list_is_noise() -> Result<(), TestError> {
    captured_row_is_noise("X03")
}

#[test]
fn wrapped_digit_lines_differ() -> Result<(), TestError> {
    captured_row_differs("X09")
}

#[test]
fn autolinked_dotted_tokens_differ() -> Result<(), TestError> {
    captured_row_differs("R12")
}

#[test]
fn code_span_indentation_is_noise() -> Result<(), TestError> {
    captured_row_is_noise("X08")
}

#[test]
fn code_span_line_ending_at_top_level_is_noise() -> Result<(), TestError> {
    captured_row_is_noise("X01")
}

#[test]
fn a_marker_swap_inside_fenced_code_differs() {
    assert_differ("```\n- a\n```\n", "```\n* a\n```\n");
}

#[test]
fn ordered_list_start_numbers_differ() {
    assert_differ("1. a\n", "3. a\n");
}

#[test]
fn strong_and_plain_differ() {
    assert_differ("**a**\n", "a\n");
}

#[test]
fn link_text_differs() {
    assert_differ("[t](u)\n", "[u](u)\n");
}

#[test]
fn bullet_and_ordered_lists_differ() {
    assert_differ("- a\n", "1. a\n");
}

#[test]
fn hard_and_soft_breaks_differ() {
    assert_differ("a\\\nb\n", "a\nb\n");
}

#[test]
fn code_span_contents_differ() {
    assert_differ("`a b`\n", "`a c`\n");
}

#[test]
fn table_alignment_differs() {
    assert_differ("| a |\n|:--|\n| 1 |\n", "| a |\n|--:|\n| 1 |\n");
}

#[test]
fn task_state_differs() {
    assert_differ("- [ ] a\n", "- [x] a\n");
}

#[test]
fn heading_levels_differ() {
    assert_differ("# a\n", "## a\n");
}

#[test]
fn paragraph_break_and_soft_break_differ_outside_a_list() {
    assert_differ("a\n\nb\n", "a\nb\n");
}

#[test]
fn strong_on_a_code_span_is_noise() {
    assert_noise("**`c`**\n", "`c`\n");
}

#[test]
fn the_captured_push_and_read_back_differ_only_by_the_meaning_changing_rows(
) -> Result<(), TestError> {
    let capture = Capture::load()?;
    assert!(!equivalent_markdown(&capture.pushed, &capture.read_back));
    let (pushed, read_back) = capture.without(&["R12", "X09"]);
    assert!(
        equivalent_markdown(&pushed, &read_back),
        "{pushed:?} and {read_back:?} should be equivalent"
    );
    Ok(())
}
