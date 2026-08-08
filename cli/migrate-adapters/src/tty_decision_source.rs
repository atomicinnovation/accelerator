//! The real-terminal `DecisionSource`: renders the prompt, reads one line
//! from stdin on a detached thread, and bounds the wait with `timeout`.
//!
//! Prompt rendering lives here rather than in `migrate-cli`'s `render`
//! module — unlike every other user-facing string, the prompt has to be
//! printed and then immediately read in the same synchronous step, so
//! splitting render-vs-read across the crate boundary the rest of this
//! engine uses would force this one seam to straddle it for no benefit.

use std::io::BufRead as _;
use std::io::Write as _;
use std::sync::mpsc;
use std::time::Duration;

use migrate::interactive::Decision;
use migrate::interactive::Transformation;
use migrate::ports::DecisionError;
use migrate::ports::DecisionSource;

pub struct TtyDecisionSource;

fn render_prompt(transformation: &Transformation) {
    println!();
    println!("{}", transformation.display);
    println!("  proposed: {}", transformation.proposed);
    println!(
        "  source: {}:{}",
        transformation.path, transformation.anchor
    );
    println!("  predicate: {}", transformation.predicate_value);
    print!("accept | skip | edit <value>: ");
    let _ = std::io::stdout().flush();
}

impl DecisionSource for TtyDecisionSource {
    fn next_decision(
        &self,
        transformation: &Transformation,
        timeout: Duration,
    ) -> Result<Decision, DecisionError> {
        render_prompt(transformation);
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
    use std::time::Duration;
    use std::time::Instant;

    use migrate::ports::DecisionError;

    use super::parse_decision;
    use super::read_one_line_bounded;
    use migrate::interactive::Decision;

    type TestError = Box<dyn std::error::Error>;

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
