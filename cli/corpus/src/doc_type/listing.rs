/// How a doc type's documents are found under its directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Listing {
    /// Files directly in the directory, and in its drafts subdirectory when
    /// it has one.
    Flat {
        drafts_subdirectory: Option<&'static str>,
        filename_filter: FilenameFilter,
    },
    /// One subdirectory per document, each holding the document as `root`.
    Nested { root: &'static str },
}

/// Which filenames in a flat listing are documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilenameFilter {
    /// A lower-case `.md` extension.
    Markdown,
    /// The work-item file convention: an `.md` extension in any case.
    WorkItem,
}
