//! Output every composition root shares.

use std::fmt::Display;
use std::sync::Once;

static PERSONAL_FILE_REPORTED: Once = Once::new();

/// Reports an ignored personal config file on stderr, once per process
/// however many times the process composes its configuration.
pub fn personal_file_warning(warning: &dyn Display) {
    report_personal_file_once(|| eprintln!("warning: {warning}"));
}

/// As [`personal_file_warning`], for a process that reports through another
/// channel. Shares the same once-guard.
pub fn report_personal_file_once(report: impl FnOnce()) {
    PERSONAL_FILE_REPORTED.call_once(report);
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::report_personal_file_once;

    #[test]
    fn the_personal_file_is_reported_once_per_process() {
        let reports = Cell::new(0);

        report_personal_file_once(|| reports.set(reports.get() + 1));
        report_personal_file_once(|| reports.set(reports.get() + 1));

        assert_eq!(reports.get(), 1);
    }
}
