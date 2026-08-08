//! The real-terminal `DecisionSource`: renders the prompt, reads one line
//! from stdin on a detached thread, and bounds the wait with `timeout`.
//!
//! Prompt rendering lives here rather than in `migrate-cli`'s `render`
//! module — unlike every other user-facing string, the prompt has to be
//! printed and then immediately read in the same synchronous step, so
//! splitting render-vs-read across the crate boundary the rest of this
//! engine uses would force this one seam to straddle it for no benefit.

use std::cell::Cell;
use std::io::BufRead as _;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use migrate::interactive::Decision;
use migrate::interactive::Transformation;
use migrate::ports::DecisionError;
use migrate::ports::DecisionSource;

/// Bound to the one session log a run's own `Interactive` entry writes to.
///
/// The compiled registry can never hold more than one `Interactive` entry,
/// so there is only ever one path to announce.
pub struct TtyDecisionSource {
    session_log_path: PathBuf,
    banner_printed: Cell<bool>,
}

impl TtyDecisionSource {
    #[must_use]
    pub const fn new(session_log_path: PathBuf) -> Self {
        Self {
            session_log_path,
            banner_printed: Cell::new(false),
        }
    }
}

/// Bash's own `render_prompt` (`interactive-lib.sh:189-224`): a one-time
/// session-log banner (so an interrupted user knows where to resume from),
/// then the three mandatory display elements (ADR-0037) plus the author's
/// own `display` content and the accept/skip/edit prompt.
fn render_prompt(
    out: &mut impl Write,
    transformation: &Transformation,
    session_log_path: &Path,
    banner_printed: &Cell<bool>,
) {
    if !banner_printed.get() {
        let _ = writeln!(
            out,
            "Session log: {}  (resume from this file by re-running \
             /accelerator:migrate)\n",
            session_log_path.display()
        );
        banner_printed.set(true);
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", transformation.display);
    let _ = writeln!(out, "  proposed: {}", transformation.proposed);
    let _ = writeln!(
        out,
        "  source: {}:{}",
        transformation.path, transformation.anchor
    );
    let _ = writeln!(out, "  predicate: {}", transformation.predicate_value);
    let _ = write!(out, "accept | skip | edit <value>: ");
    let _ = out.flush();
}

impl DecisionSource for TtyDecisionSource {
    fn next_decision(
        &self,
        transformation: &Transformation,
        timeout: Duration,
    ) -> Result<Decision, DecisionError> {
        render_prompt(
            &mut std::io::stdout(),
            transformation,
            &self.session_log_path,
            &self.banner_printed,
        );
        read_one_line_bounded(std::io::stdin(), timeout)
    }
}

/// The spawned-thread-plus-`recv_timeout` core, generic over the reader so a
/// test can drive it against a real OS pipe rather than the process's own
/// stdin (which nothing in `std` lets a test redirect).
fn read_one_line_bounded(
    reader: impl std::io::Read + Send + 'static,
    timeout: Duration,
) -> Result<Decision, DecisionError> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let read = std::io::BufReader::new(reader).read_line(&mut line);
        let _ = sender.send(match read {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line),
        });
    });

    match receiver.recv_timeout(timeout) {
        Ok(Some(line)) => Ok(parse_decision(&line)),
        Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(DecisionError::Eof)
        }
        Err(mpsc::RecvTimeoutError::Timeout) => Err(DecisionError::Timeout),
    }
}

fn parse_decision(line: &str) -> Decision {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    if trimmed == "accept" {
        Decision::Accept
    } else if trimmed == "skip" {
        Decision::Skip
    } else if let Some(value) = trimmed.strip_prefix("edit ") {
        Decision::Edit(value.to_owned())
    } else {
        Decision::Edit(trimmed.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::path::Path;
    use std::time::Duration;
    use std::time::Instant;

    use migrate::interactive::Transformation;
    use migrate::ports::DecisionError;

    use super::parse_decision;
    use super::read_one_line_bounded;
    use super::render_prompt;
    use migrate::interactive::Decision;

    type TestError = Box<dyn std::error::Error>;

    fn transformation() -> Transformation {
        Transformation {
            key: "meta/work/0001.md#body/relates_to".to_owned(),
            path: "meta/work/0001.md".to_owned(),
            anchor: "body/relates_to".to_owned(),
            proposed: "relates_to=work-item:0042".to_owned(),
            predicate_value: "ambiguous".to_owned(),
            display: "Proposed linkage: relates_to: \"work-item:0042\""
                .to_owned(),
            extras: Vec::new(),
        }
    }

    #[test]
    fn the_session_log_banner_prints_once_before_the_first_prompt(
    ) -> Result<(), TestError> {
        let path =
            Path::new("/repo/.accelerator/state/migrations-0007-session.jsonl");
        let banner_printed = Cell::new(false);
        let mut out = Vec::new();

        render_prompt(&mut out, &transformation(), path, &banner_printed);
        let after_first = String::from_utf8(out.clone())?;
        render_prompt(&mut out, &transformation(), path, &banner_printed);
        let after_second = String::from_utf8(out)?;

        assert!(
            after_first.starts_with(
                "Session log: /repo/.accelerator/state/migrations-0007-session.jsonl  \
                 (resume from this file by re-running /accelerator:migrate)\n"
            ),
            "{after_first}"
        );
        assert_eq!(
            after_second.matches("Session log:").count(),
            1,
            "{after_second}"
        );
        Ok(())
    }

    #[test]
    fn the_prompt_shows_the_three_mandatory_elements_and_the_display_content(
    ) -> Result<(), TestError> {
        let path =
            Path::new("/repo/.accelerator/state/migrations-0007-session.jsonl");
        let banner_printed = Cell::new(true);
        let mut out = Vec::new();

        render_prompt(&mut out, &transformation(), path, &banner_printed);
        let rendered = String::from_utf8(out)?;

        assert!(
            rendered
                .contains("Proposed linkage: relates_to: \"work-item:0042\""),
            "{rendered}"
        );
        assert!(
            rendered.contains("proposed: relates_to=work-item:0042"),
            "{rendered}"
        );
        assert!(
            rendered.contains("source: meta/work/0001.md:body/relates_to"),
            "{rendered}"
        );
        assert!(rendered.contains("predicate: ambiguous"), "{rendered}");
        assert!(
            rendered.contains("accept | skip | edit <value>: "),
            "the inline-help prompt line must be shown: {rendered}"
        );
        Ok(())
    }

    #[test]
    fn a_pipe_that_never_writes_times_out_within_bound_plus_two_seconds(
    ) -> Result<(), TestError> {
        let (reader, writer) = std::io::pipe()?;
        let started = Instant::now();

        let result = read_one_line_bounded(reader, Duration::from_millis(50));

        assert!(matches!(result, Err(DecisionError::Timeout)));
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "took {:?}",
            started.elapsed()
        );
        drop(writer);
        Ok(())
    }

    #[test]
    fn a_closed_pipe_reports_eof() -> Result<(), TestError> {
        let (reader, writer) = std::io::pipe()?;
        drop(writer);

        let result = read_one_line_bounded(reader, Duration::from_secs(1));

        assert!(matches!(result, Err(DecisionError::Eof)));
        Ok(())
    }

    #[test]
    fn a_written_line_is_read_before_the_bound() -> Result<(), TestError> {
        use std::io::Write as _;
        let (reader, mut writer) = std::io::pipe()?;
        writeln!(writer, "accept")?;

        let result = read_one_line_bounded(reader, Duration::from_secs(1));

        assert!(matches!(result, Ok(Decision::Accept)));
        Ok(())
    }

    #[test]
    fn accept_and_skip_parse_as_the_named_decisions() {
        assert!(matches!(parse_decision("accept\n"), Decision::Accept));
        assert!(matches!(parse_decision("skip\n"), Decision::Skip));
    }

    #[test]
    fn edit_with_a_value_parses_as_edit() {
        let decision = parse_decision("edit hello world\n");
        assert!(matches!(&decision, Decision::Edit(_)), "{decision:?}");
        if let Decision::Edit(value) = decision {
            assert_eq!(value, "hello world");
        }
    }
}
