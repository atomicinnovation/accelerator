//! The `version` domain core and the two ports it speaks through.
//!
//! Depends on no infrastructure — cargo-pup enforces that inward direction.

/// Build-time facts the core requires — an outbound/driven port.
pub trait BuildMetadata {
    fn crate_version(&self) -> &'static str;
    fn commit_sha(&self) -> &'static str;
    fn build_date(&self) -> &'static str;
    fn target_triple(&self) -> &'static str;
}

/// What `version` produces — a value object, no presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionReport {
    pub version: String,
    pub commit_sha: String,
    pub build_date: String,
    pub target_triple: String,
}

/// The operation the core offers callers — an inbound/driving port.
pub trait ReportVersion {
    fn report(&self) -> VersionReport;
}

/// The application service — depends only on the outbound port.
pub struct VersionReporter<M: BuildMetadata> {
    metadata: M,
}

impl<M: BuildMetadata> VersionReporter<M> {
    pub const fn new(metadata: M) -> Self {
        Self { metadata }
    }
}

impl<M: BuildMetadata> ReportVersion for VersionReporter<M> {
    fn report(&self) -> VersionReport {
        VersionReport {
            version: self.metadata.crate_version().to_owned(),
            commit_sha: self.metadata.commit_sha().to_owned(),
            build_date: self.metadata.build_date().to_owned(),
            target_triple: self.metadata.target_triple().to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BuildMetadata, ReportVersion, VersionReporter};

    struct FakeBuildMetadata;

    impl BuildMetadata for FakeBuildMetadata {
        fn crate_version(&self) -> &'static str {
            "9.9.9-fake"
        }
        fn commit_sha(&self) -> &'static str {
            "0badc0de"
        }
        fn build_date(&self) -> &'static str {
            "2000-01-01T00:00:00Z"
        }
        fn target_triple(&self) -> &'static str {
            "fake-unknown-target"
        }
    }

    #[test]
    fn reports_the_injected_build_metadata() {
        let report = VersionReporter::new(FakeBuildMetadata).report();
        assert_eq!(report.version, "9.9.9-fake");
        assert_eq!(report.commit_sha, "0badc0de");
        assert_eq!(report.build_date, "2000-01-01T00:00:00Z");
        assert_eq!(report.target_triple, "fake-unknown-target");
    }
}
