//! Whether a corpus's work-item IDs can be trusted: every item under the ID
//! the scheme gives it, and every work-item file readable for its IDs.

use std::fmt::Display;
use std::fmt::Formatter;
use std::path::Path;

use crate::convergence::refuse_divergence;
use crate::convergence::RekeyRequired;
use crate::id_scheme::IdScheme;
use crate::well_formed::refuse_malformed;
use crate::well_formed::MalformedItems;
use crate::work_item_files::identities;
use crate::work_item_files::WorkItemListing;

/// Why a corpus's IDs cannot be trusted, which commands that read or write
/// IDs refuse to run over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UntrustedCorpus {
    RekeyRequired(RekeyRequired),
    Malformed(MalformedItems),
}

impl UntrustedCorpus {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::RekeyRequired(_) => RekeyRequired::CODE,
            Self::Malformed(_) => MalformedItems::CODE,
        }
    }

    /// The refusal with every path relative to `root`.
    #[must_use]
    pub fn relative_to(self, root: &Path) -> Self {
        match self {
            Self::RekeyRequired(refusal) => {
                Self::RekeyRequired(refusal.relative_to(root))
            }
            Self::Malformed(refusal) => {
                Self::Malformed(refusal.relative_to(root))
            }
        }
    }
}

impl Display for UntrustedCorpus {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RekeyRequired(refusal) => refusal.fmt(formatter),
            Self::Malformed(refusal) => refusal.fmt(formatter),
        }
    }
}

impl std::error::Error for UntrustedCorpus {}

/// Refuses a corpus holding an item re-key would move, then one holding a
/// work-item file whose IDs cannot be read.
///
/// # Errors
///
/// [`UntrustedCorpus`] naming the first reason the corpus cannot be
/// trusted.
pub fn refuse_untrusted(
    scheme: &IdScheme,
    listing: &WorkItemListing,
) -> Result<(), UntrustedCorpus> {
    refuse_divergence(scheme, &identities(&listing.files))
        .map_err(UntrustedCorpus::RekeyRequired)?;
    refuse_malformed(scheme, listing).map_err(UntrustedCorpus::Malformed)
}

#[cfg(test)]
// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use super::refuse_untrusted;
    use super::UntrustedCorpus;
    use crate::id_pattern::IdKey;
    use crate::id_pattern::IdPattern;
    use crate::id_scheme::IdScheme;
    use crate::work_item_files::WorkItemFile;
    use crate::work_item_files::WorkItemListing;

    fn listing_of(files: &[WorkItemFile]) -> WorkItemListing {
        WorkItemListing {
            files: files.to_vec(),
            unreadable: Vec::new(),
        }
    }

    fn keyed() -> IdScheme {
        IdScheme::new(
            IdPattern::parse("{key}-{number:04d}")
                .unwrap_or_else(|_| unreachable!()),
            IdKey::parse("PP").ok(),
        )
    }

    fn file(name: &str, id: &str) -> WorkItemFile {
        WorkItemFile {
            path: PathBuf::from(format!("/repo/meta/work/{name}")),
            content: format!("---\nid: \"{id}\"\ntitle: \"T\"\n---\n"),
        }
    }

    #[test]
    fn a_converged_well_formed_corpus_is_trusted() {
        let files = [file("PP-0001-a.md", "PP-0001")];

        assert_eq!(refuse_untrusted(&keyed(), &listing_of(&files)), Ok(()));
    }

    #[test]
    fn a_corpus_needing_a_re_key_is_refused_before_a_malformed_item() {
        let files = [file("PP-0002.md", "PP-0002"), file("PP-1-a.md", "PP-1")];

        let refusal = refuse_untrusted(&keyed(), &listing_of(&files))
            .err()
            .unwrap_or_else(|| unreachable!("the corpus is untrusted"));

        assert!(matches!(refusal, UntrustedCorpus::RekeyRequired(_)));
        assert_eq!(refusal.code(), "E_REKEY_REQUIRED");
    }

    #[test]
    fn a_converged_corpus_with_a_malformed_item_is_refused_naming_it() {
        let files = [file("PP-0002.md", "PP-0002")];

        let refusal = refuse_untrusted(&keyed(), &listing_of(&files))
            .err()
            .unwrap_or_else(|| unreachable!("the corpus is untrusted"))
            .relative_to(Path::new("/repo"));

        assert_eq!(refusal.code(), "E_WORK_ITEM_MALFORMED");
        assert!(
            refusal.to_string().starts_with(
                "E_WORK_ITEM_MALFORMED: meta/work/PP-0002.md has `id`"
            ),
            "{refusal}"
        );
    }
}
