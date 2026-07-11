//! The clock port and the artifact-metadata output type.

/// A filename-timestamp format, one per subsumed metadata helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilenameTimestampFormat {
    DateTimeUnderscored,
    DateOnly,
    CompactTime,
}

/// The wall-clock seam: UTC ISO for the datetime line, host-local for the
/// filename timestamp. Faked in tests so every derived field is deterministic.
pub trait Clock {
    fn now_utc_iso(&self) -> String;
    fn filename_timestamp(&self, format: FilenameTimestampFormat) -> String;
}

/// The facts an artifact-metadata block carries. `repository_name`/`revision`
/// are absent outside a VCS checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactMetadata {
    pub datetime_utc: String,
    pub filename_timestamp: String,
    pub repository_name: Option<String>,
    pub revision: Option<String>,
}
