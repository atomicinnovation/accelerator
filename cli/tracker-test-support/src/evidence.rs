//! The reduced conformance-evidence format shared by both provider contract
//! harnesses, plus the guard that keeps a committed evidence file free of
//! payloads and secrets.
//!
//! A live contract run exercises `show` and `fetch_all` over real issues, so a
//! verbatim transcript would carry issue keys, summaries, ADF bodies, account
//! ids, email addresses and site hostnames — and possibly an echoed
//! `Authorization` in a failure diagnostic — into the repository permanently.
//! The evidence is therefore reduced to test name, outcome, count and duration
//! by construction: [`render`] emits only those fields, and [`is_reduced`]
//! refuses a committed file carrying anything else.
//!
//! The guard is structural first: every line must be a header comment over a
//! restricted charset or a record over the fixed `name PASS|FAIL count Nms`
//! grammar, so an unknown or rotated token format cannot appear at all. A
//! denylist of known secret shapes is the second net.

use std::time::Duration;

/// One conformance property's outcome, reduced to the fields safe to commit.
pub struct EvidenceRecord {
    pub name: String,
    pub passed: bool,
    pub count: usize,
    pub duration: Duration,
}

const HARNESS: &str = "test:integration:tracker-contract";

const fn outcome(passed: bool) -> &'static str {
    if passed {
        "PASS"
    } else {
        "FAIL"
    }
}

fn push_record(
    out: &mut String,
    name: &str,
    passed: bool,
    count: usize,
    ms: u128,
) {
    out.push_str(name);
    out.push(' ');
    out.push_str(outcome(passed));
    out.push(' ');
    out.push_str(&count.to_string());
    out.push(' ');
    out.push_str(&ms.to_string());
    out.push_str("ms\n");
}

/// The reduced record for a provider's conformance run, with a `total` line
/// folding the individual outcomes, counts and durations.
#[must_use]
pub fn render(
    provider: &str,
    date: Option<&str>,
    records: &[EvidenceRecord],
) -> String {
    let mut out = String::new();
    out.push_str("# RemoteTracker contract conformance evidence\n");
    out.push_str("# provider: ");
    out.push_str(provider);
    out.push('\n');
    if let Some(date) = date {
        out.push_str("# date: ");
        out.push_str(date);
        out.push('\n');
    }
    out.push_str("# harness: ");
    out.push_str(HARNESS);
    out.push('\n');

    let mut all_passed = true;
    let mut total_count = 0usize;
    let mut total_ms = 0u128;
    for record in records {
        all_passed &= record.passed;
        total_count += record.count;
        total_ms += record.duration.as_millis();
        push_record(
            &mut out,
            &record.name,
            record.passed,
            record.count,
            record.duration.as_millis(),
        );
    }
    push_record(&mut out, "total", all_passed, total_count, total_ms);
    out
}

const SECRET_SHAPES: [&str; 3] = ["ATATT", "lin_api_", "Bearer "];

fn comment_is_safe(line: &str) -> bool {
    line.chars().all(|character| {
        character.is_ascii_alphanumeric()
            || matches!(character, ' ' | '#' | ':' | '.' | '_' | '/' | '-')
    })
}

fn record_is_wellformed(line: &str) -> bool {
    let fields: Vec<&str> = line.split(' ').collect();
    let [name, status, count, duration] = fields.as_slice() else {
        return false;
    };
    let name_ok = !name.is_empty()
        && name.chars().all(|c| c.is_ascii_lowercase() || c == '_');
    let status_ok = *status == "PASS" || *status == "FAIL";
    let count_ok =
        !count.is_empty() && count.chars().all(|c| c.is_ascii_digit());
    let duration_ok = duration.strip_suffix("ms").is_some_and(|ms| {
        !ms.is_empty() && ms.chars().all(|c| c.is_ascii_digit())
    });
    name_ok && status_ok && count_ok && duration_ok
}

/// Reasons a committed evidence file is not the reduced form. Empty `Ok`
/// means the file carries only header comments and well-formed records and
/// matches no known secret shape.
///
/// # Errors
///
/// Returns every violation found rather than the first, so a regenerated
/// file's problems surface in one pass.
pub fn is_reduced(contents: &str) -> Result<(), Vec<String>> {
    let mut violations = Vec::new();

    for shape in SECRET_SHAPES {
        if contents.contains(shape) {
            violations
                .push(format!("secret-shaped substring present: {shape}"));
        }
    }
    if contents.contains('@') {
        violations.push(
            "an '@' is present — evidence carries no addresses".to_owned(),
        );
    }

    for line in contents.lines() {
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            if !comment_is_safe(line) {
                violations
                    .push(format!("comment outside the safe set: {line}"));
            }
            continue;
        }
        if !record_is_wellformed(line) {
            violations.push(format!("line is not a reduced record: {line}"));
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::is_reduced;
    use super::render;
    use super::EvidenceRecord;
    use std::time::Duration;

    fn sample() -> Vec<EvidenceRecord> {
        vec![
            EvidenceRecord {
                name: "create_then_show_round_trips".to_owned(),
                passed: true,
                count: 1,
                duration: Duration::from_millis(123),
            },
            EvidenceRecord {
                name: "fetch_all_partitions_totally".to_owned(),
                passed: true,
                count: 3,
                duration: Duration::from_millis(456),
            },
        ]
    }

    #[test]
    fn a_rendered_report_is_accepted_by_the_guard() {
        let rendered = render("jira", Some("2026-08-18"), &sample());
        assert_eq!(is_reduced(&rendered), Ok(()), "{rendered}");
    }

    #[test]
    fn the_total_line_folds_counts_and_durations() {
        let rendered = render("linear", None, &sample());
        assert!(rendered.contains("total PASS 4 579ms\n"), "{rendered}");
        assert!(!rendered.contains("# date:"), "no date line when absent");
    }

    #[test]
    fn a_failing_record_marks_the_total_failed() {
        let records = vec![EvidenceRecord {
            name: "a_failing_read_is_retryable".to_owned(),
            passed: false,
            count: 1,
            duration: Duration::from_millis(30),
        }];
        let rendered = render("jira", None, &records);
        assert!(rendered.contains("total FAIL 1 30ms\n"), "{rendered}");
    }

    #[test]
    fn an_email_address_is_a_violation() {
        let dirty = "# provider: jira\n# reporter: alice@example.com\n";
        assert!(is_reduced(dirty).is_err());
    }

    #[test]
    fn a_bearer_token_is_a_violation() {
        let dirty = "# provider: jira\nAuthorization Bearer abc123 1ms\n";
        assert!(is_reduced(dirty).is_err());
    }

    #[test]
    fn a_jira_token_prefix_is_a_violation() {
        let dirty = "# provider: jira\n# note ATATT3xFfGF0 1ms\n";
        assert!(is_reduced(dirty).is_err());
    }

    #[test]
    fn a_linear_token_prefix_is_a_violation() {
        let dirty = "# lin_api_0123456789\n";
        assert!(is_reduced(dirty).is_err());
    }

    #[test]
    fn a_payload_line_is_not_a_reduced_record() {
        let dirty = "# provider: jira\nsummary: fix the widget crash\n";
        assert!(is_reduced(dirty).is_err());
    }

    #[test]
    fn a_malformed_duration_is_rejected() {
        assert!(
            is_reduced("create_then_show_round_trips PASS 1 123\n").is_err()
        );
        assert!(is_reduced("create_then_show_round_trips PASS 1 ms\n").is_err());
    }

    #[test]
    fn an_uppercase_property_name_is_rejected() {
        assert!(is_reduced("CreateThenShow PASS 1 1ms\n").is_err());
    }
}
