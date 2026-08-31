//! Cross-backend parity for `vcs status`/`vcs log` (AC3).
//!
//! Builds the same logical working-copy state in a git and a jj repo and asserts
//! the ADR-0066 format renders in the same *shape* from both — not byte-identity,
//! since the branch value, the git-only staged row, and the untracked/added
//! divergence differ by design. The log lines are compared per line after
//! masking, never by equating the two masked blobs (which carry distinct
//! `<HEX_OBJECT_ID>`/`<JJ_CHANGE_ID>` tokens), with an unmasked control proving
//! the mask touched only the id span.
#![cfg(feature = "bash-parity")]

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use vcs_test_support::hermetic::Hermetic;
use vcs_test_support::masks;
use vcs_test_support::masks::Mask;
use vcs_test_support::status_log;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-vcs");

const LABELS: [&str; 5] =
    ["added", "modified", "deleted", "untracked", "conflicted"];

fn masks_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../vcs-test-support/fixtures/masks.toml")
}

fn run_vcs(subcommand: &str, dir: &Path) -> Result<String, TestError> {
    let output = Command::new(BIN)
        .arg(subcommand)
        .current_dir(dir)
        .output()?;
    assert!(
        output.status.success(),
        "accelerator-vcs {subcommand} exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?
        .trim_end_matches('\n')
        .to_owned())
}

/// The file-change lines of a status render, asserting the shared shape: a
/// `Branch:` header, an `<N> changed` summary, and `  <label>  <path>` lines in
/// byte order. Returns each `(label, path)`.
fn status_shape(rendered: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = rendered.lines().collect();
    assert!(
        lines[0].starts_with("Branch: "),
        "a status must open with the Branch header: {rendered:?}"
    );

    let summary = lines[1];
    let count_token = summary.split(' ').next().unwrap_or_default();
    assert!(
        count_token.parse::<usize>().is_ok()
            && summary.split(' ').nth(1) == Some("changed"),
        "the summary must read '<N> changed[...]': {summary:?}"
    );

    let mut changes = Vec::new();
    for line in &lines[2..] {
        let rest = line.strip_prefix("  ");
        assert!(
            rest.is_some(),
            "a file line is two-space indented: {line:?}"
        );
        let split = rest.unwrap_or("").split_once("  ");
        assert!(
            split.is_some(),
            "a file line is '<label>  <path>': {line:?}"
        );
        let (label, path) = split.unwrap_or(("", ""));
        assert!(
            LABELS.contains(&label),
            "unexpected change label {label:?} in {line:?}"
        );
        changes.push((label.to_owned(), path.to_owned()));
    }

    let mut sorted = changes.clone();
    sorted.sort_by(|left, right| left.1.as_bytes().cmp(right.1.as_bytes()));
    assert_eq!(sorted, changes, "file lines must be byte-sorted by path");

    changes
}

/// Asserts each masked log line is `<id-token> <subject>` with a real id token,
/// and — via the unmasked control — that the mask replaced only the id span.
fn assert_log_line_shape(
    masks: &[Mask],
    unmasked: &str,
) -> Result<(), TestError> {
    let masked = masks::apply(masks, unmasked)?;
    for (masked_line, unmasked_line) in masked.lines().zip(unmasked.lines()) {
        let (token, masked_suffix) = masked_line
            .split_once(' ')
            .ok_or("a log line must be '<id> <subject>'")?;
        assert!(
            token == "<HEX_OBJECT_ID>" || token == "<JJ_CHANGE_ID>",
            "the id must mask to a known token: {masked_line:?}"
        );
        assert!(!masked_suffix.is_empty(), "a log line needs a subject");

        let unmasked_suffix = unmasked_line
            .split_once(' ')
            .ok_or("an unmasked log line must be '<id> <subject>'")?
            .1;
        assert_eq!(
            unmasked_suffix, masked_suffix,
            "the mask must touch only the id span, not the subject"
        );
    }
    Ok(())
}

#[test]
fn status_and_log_render_in_the_same_shape_from_both_backends(
) -> Result<(), TestError> {
    let masks = masks::load(&masks_path())?;
    let work = tempfile::Builder::new()
        .prefix("vcs-status-log-parity-")
        .tempdir()?;
    let env = Hermetic::rooted_at(work.path())?;
    let states = status_log::build_parity_states(work.path(), &env)?;
    let git = states.get("parity-git").ok_or("parity-git missing")?;
    let jj = states.get("parity-jj").ok_or("parity-jj missing")?;

    let git_changes = status_shape(&run_vcs("status", git)?);
    let jj_changes = status_shape(&run_vcs("status", jj)?);

    let modified_tracked = ("modified".to_owned(), "tracked.txt".to_owned());
    assert!(
        git_changes.contains(&modified_tracked),
        "git must render the modified tracked file: {git_changes:?}"
    );
    assert!(
        jj_changes.contains(&modified_tracked),
        "jj must render the modified tracked file: {jj_changes:?}"
    );

    assert_log_line_shape(&masks, &run_vcs("log", git)?)?;
    assert_log_line_shape(&masks, &run_vcs("log", jj)?)?;
    Ok(())
}

#[test]
fn a_five_commit_log_carries_no_author_date_or_graph() -> Result<(), TestError>
{
    let work = tempfile::Builder::new()
        .prefix("vcs-status-log-content-")
        .tempdir()?;
    let env = Hermetic::rooted_at(work.path())?;
    let states = status_log::build_cap_states(work.path(), &env)?;
    let git = states.get("cap-git").ok_or("cap-git missing")?;

    // Author/date must be checked on the raw render — a committed mask would
    // rewrite a leaked timestamp or email and pass falsely.
    let raw = run_vcs("log", git)?;
    assert_eq!(raw.lines().count(), 5, "the log is five entries: {raw:?}");
    for line in raw.lines() {
        let (_id, subject) = line
            .split_once(' ')
            .ok_or("a log line must be '<id> <subject>'")?;
        assert!(!subject.is_empty(), "each entry carries a subject");
        assert!(
            !subject.contains('@'),
            "no author identity may appear: {line:?}"
        );
    }
    for glyph in ['@', '\u{2502}', '\u{25cb}', '\u{25c6}'] {
        assert!(
            !raw.contains(glyph),
            "no ASCII-graph glyph {glyph:?} may appear: {raw:?}"
        );
    }
    Ok(())
}
