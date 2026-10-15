//! Raw tracker content an earlier release's pull left in a local file.

/// What a local body holds in place of Markdown.
///
/// Sending either would replace the tracker's description with its own
/// document as text, so a send is refused while the residue remains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullResidue {
    /// The tracker's stored document, differing from the current remote.
    TrackerWireDocument,
    /// The tracker's stored document, equal to the current remote.
    VerbatimRemote,
}
