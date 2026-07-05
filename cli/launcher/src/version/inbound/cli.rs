//! The inbound adapter for `version`: renders the report, drives the port.

use crate::version::core::{ReportVersion, VersionReport};

/// Renders a [`VersionReport`] as the human-facing `version` output.
#[must_use]
pub fn render(report: &VersionReport) -> String {
    format!(
        "accelerator {}\ncommit: {}\nbuilt:  {}\ntarget: {}",
        report.version,
        report.commit_sha,
        report.build_date,
        report.target_triple,
    )
}

/// Drives the inbound port and prints the rendered `version` output.
pub fn report(reporter: &impl ReportVersion) {
    tracing::debug!("reporting version");
    println!("{}", render(&reporter.report()));
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::version::core::VersionReport;

    fn sample_report() -> VersionReport {
        VersionReport {
            version: "1.2.3".to_owned(),
            commit_sha: "abc123".to_owned(),
            build_date: "2020-01-02T03:04:05Z".to_owned(),
            target_triple: "x86_64-unknown-linux-gnu".to_owned(),
        }
    }

    #[test]
    fn render_produces_four_prefixed_lines_in_order() {
        assert_eq!(
            render(&sample_report()),
            "accelerator 1.2.3\n\
             commit: abc123\n\
             built:  2020-01-02T03:04:05Z\n\
             target: x86_64-unknown-linux-gnu"
        );
    }
}
