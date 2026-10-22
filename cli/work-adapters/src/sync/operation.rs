//! What a sync was doing when it failed, spelled once for every report.

/// One step of a sync run, named by the keyword its failures report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOperation {
    LockLocal,
    ReadLocal,
    RecheckLocal,
    IdentifyLocal,
    ClassifyLocal,
    ParseLocal,
    KeepEdits,
    HashLocal,
    WriteLocal,
    AuthorLocal,
    MarkerWrite,
    LinkExternalId,
    BaselineSet,
    Discover,
    Configure,
    AdoptRecord,
    Pull,
    Show,
    Update,
    Retitle,
    Create,
    Adopt,
}

impl SyncOperation {
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::LockLocal => "lock-local",
            Self::ReadLocal => "read-local",
            Self::RecheckLocal => "recheck-local",
            Self::IdentifyLocal => "identify-local",
            Self::ClassifyLocal => "classify-local",
            Self::ParseLocal => "parse-local",
            Self::KeepEdits => "keep-edits",
            Self::HashLocal => "hash-local",
            Self::WriteLocal => "write-local",
            Self::AuthorLocal => "author-local",
            Self::MarkerWrite => "marker-write",
            Self::LinkExternalId => "link-external-id",
            Self::BaselineSet => "baseline-set",
            Self::Discover => "discover",
            Self::Configure => "configure",
            Self::AdoptRecord => "adopt-record",
            Self::Pull => "pull",
            Self::Show => "show",
            Self::Update => "update",
            Self::Retitle => "retitle",
            Self::Create => "create",
            Self::Adopt => "adopt",
        }
    }
}

impl std::fmt::Display for SyncOperation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.keyword())
    }
}

#[cfg(test)]
mod tests {
    use super::SyncOperation;

    #[test]
    fn each_operation_is_named_by_its_keyword() {
        let keywords = [
            (SyncOperation::LockLocal, "lock-local"),
            (SyncOperation::ReadLocal, "read-local"),
            (SyncOperation::RecheckLocal, "recheck-local"),
            (SyncOperation::IdentifyLocal, "identify-local"),
            (SyncOperation::ClassifyLocal, "classify-local"),
            (SyncOperation::ParseLocal, "parse-local"),
            (SyncOperation::KeepEdits, "keep-edits"),
            (SyncOperation::HashLocal, "hash-local"),
            (SyncOperation::WriteLocal, "write-local"),
            (SyncOperation::AuthorLocal, "author-local"),
            (SyncOperation::MarkerWrite, "marker-write"),
            (SyncOperation::LinkExternalId, "link-external-id"),
            (SyncOperation::BaselineSet, "baseline-set"),
            (SyncOperation::Discover, "discover"),
            (SyncOperation::Configure, "configure"),
            (SyncOperation::AdoptRecord, "adopt-record"),
            (SyncOperation::Pull, "pull"),
            (SyncOperation::Show, "show"),
            (SyncOperation::Update, "update"),
            (SyncOperation::Retitle, "retitle"),
            (SyncOperation::Create, "create"),
            (SyncOperation::Adopt, "adopt"),
        ];
        for (operation, keyword) in keywords {
            assert_eq!(operation.keyword(), keyword);
            assert_eq!(operation.to_string(), keyword);
        }
    }
}
