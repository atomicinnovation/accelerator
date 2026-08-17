//! Tree-artifact resolution: its failure taxonomy and the ports it speaks
//! through.
//!
//! Two integrity models live in this crate, and the difference is deliberate
//! rather than an oversight. A single-file sub-binary is re-verified on every
//! exec, so a corrupted cache entry self-heals; a directory tree is verified
//! once at materialisation and thereafter trusted, because re-hashing ~294MB on
//! every one of a crawl's 100-200 invocations would cost more than the artifact
//! saves. What replaces self-healing is the sealed mode, the signed
//! attestation, and an explicit repair path.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::path::PathBuf;
use std::time::Duration;

/// Whether a failure is evidence of tampering or merely of an unavailable
/// artifact.
///
/// The distinction decides real behaviour rather than only wording: only
/// `Failed` is swallowable under `--fail-safe`, so a hostile archive stops a
/// crawl while local damage degrades it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    Refusal,
    Failed,
}

/// Why a tree could not be resolved, materialised or verified.
///
/// Nested under [`super::ResolutionError`] rather than flattened into it: one
/// enum spanning both resolution paths would reach twenty variants and force
/// every consumer to reason about variants that cannot arise on its path.
#[derive(Debug)]
pub enum TreeError {
    Attestation {
        detail: String,
    },
    UnexpectedDigest {
        artifact: String,
        expected: String,
        found: String,
    },
    PathEscape {
        entry: String,
    },
    Extraction {
        detail: String,
    },
    Seal {
        detail: String,
    },
    TableMissing,
    LayoutUnsupported {
        found: u32,
        supported: u32,
    },
    Pointer {
        detail: String,
    },
    Lease {
        detail: String,
    },
    DiskShortfall {
        needed: u64,
        available: u64,
    },
    MaterialisationInProgress {
        waited: Duration,
    },
}

impl TreeError {
    /// The class this failure carries into `kernel::Error`.
    ///
    /// The match is deliberately wildcard-free, so a variant added without a
    /// classification does not compile. The two hand-maintained lists that pin
    /// the single-file mapping have no such link to their enum, which is how a
    /// new variant there could ship unclassified.
    #[must_use]
    pub const fn class(&self) -> ErrorClass {
        match *self {
            // Evidence of tampering: a signature or field mismatch, an archive
            // writing outside its root, a rejected entry, a tree that cannot be
            // sealed, or an archive with no table to verify against at all.
            Self::Attestation { .. }
            | Self::UnexpectedDigest { .. }
            | Self::PathEscape { .. }
            | Self::Extraction { .. }
            | Self::Seal { .. }
            | Self::TableMissing => ErrorClass::Refusal,
            // Recoverable or environmental: re-materialisation, a remediation
            // the user can perform, or another process already succeeding.
            Self::LayoutUnsupported { .. }
            | Self::Pointer { .. }
            | Self::Lease { .. }
            | Self::DiskShortfall { .. }
            | Self::MaterialisationInProgress { .. } => ErrorClass::Failed,
        }
    }
}

impl Display for TreeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Attestation { detail } => {
                write!(
                    formatter,
                    "the artifact's attestation is not trusted: {detail}"
                )
            }
            Self::UnexpectedDigest {
                artifact,
                expected,
                found,
            } => write!(
                formatter,
                "the release names digest {found} for the {artifact} artifact, \
                 but this launcher expects {expected}"
            ),
            Self::PathEscape { entry } => write!(
                formatter,
                "archive entry '{entry}' would write outside the tree"
            ),
            Self::Extraction { detail } => {
                write!(
                    formatter,
                    "the archive could not be extracted: {detail}"
                )
            }
            Self::Seal { detail } => {
                write!(
                    formatter,
                    "the extracted tree could not be sealed: {detail}"
                )
            }
            Self::TableMissing => write!(
                formatter,
                "the archive carries no file table, so nothing in it can be \
                 verified"
            ),
            Self::LayoutUnsupported { found, supported } => write!(
                formatter,
                "the cached tree uses layout version {found}, and this \
                 launcher understands {supported}"
            ),
            Self::Pointer { detail } => {
                write!(
                    formatter,
                    "the cached tree pointer is unusable: {detail}"
                )
            }
            Self::Lease { detail } => {
                write!(
                    formatter,
                    "the tree's in-use lease could not be held: {detail}"
                )
            }
            Self::DiskShortfall { needed, available } => write!(
                formatter,
                "materialising needs {needed} bytes and {available} are free"
            ),
            Self::MaterialisationInProgress { waited } => write!(
                formatter,
                "another process is still materialising this artifact after \
                 {waited:?}"
            ),
        }
    }
}

impl std::error::Error for TreeError {}

/// A materialised tree, ready to be handed to a consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedTree {
    pub artifact: String,
    pub path: PathBuf,
    pub lease_path: PathBuf,
    pub digest: String,
}

/// A lease held on a generation for as long as this value lives.
///
/// The reaper's liveness oracle is the kernel's own view of who holds the lock,
/// so the holder must be a process that outlives the resolution — which is why
/// taking it is named in the port rather than hidden inside a lookup.
pub trait HeldLease {}

/// A tree together with the lease pinning it against reclamation.
pub struct AcquiredTree {
    pub tree: SealedTree,
    pub lease: Box<dyn HeldLease>,
}

/// How one entry disagrees with the table describing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Discrepancy {
    Missing,
    Unexpected,
    Size { expected: u64, found: u64 },
    Mode { expected: u32, found: u32 },
    Digest,
    LinkTarget { expected: String, found: String },
}

/// What a verification walk found, per entry rather than as a bare pass/fail,
/// so the output diagnoses as well as detects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeReport {
    pub artifact: String,
    pub findings: Vec<(String, Discrepancy)>,
}

impl TreeReport {
    #[must_use]
    pub const fn is_sound(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Resolve an already-materialised tree — a driven port, local only.
///
/// This is the **only** tree port the dispatch path may call. Materialisation
/// is deliberately not one argument away from it: the whole design rests on a
/// dispatch never reaching the network.
pub trait AcquireSealedTree {
    /// Look up the tree without pinning it, for diagnostics and reclamation
    /// decisions that must not keep a generation alive merely by inspecting it.
    ///
    /// # Errors
    ///
    /// A [`TreeError`] only where the state is actively wrong; an absent or
    /// unusable tree is `Ok(None)`, because "not materialised yet" is the
    /// normal state rather than a failure.
    fn query(&self, artifact: &str) -> Result<Option<SealedTree>, TreeError>;

    /// Resolve the tree and hold a shared lease on it.
    ///
    /// # Errors
    ///
    /// As [`AcquireSealedTree::query`].
    fn acquire(
        &self,
        artifact: &str,
    ) -> Result<Option<AcquiredTree>, TreeError>;
}

/// Fetch, verify, extract and seal a tree — a driven port reaching the network.
pub trait MaterialiseTree {
    /// # Errors
    ///
    /// A [`TreeError`] describing why materialisation did not complete.
    fn materialise(&self, artifact: &str) -> Result<SealedTree, TreeError>;
}

/// Walk a sealed tree against its own file table — a driven port, read-only.
pub trait VerifyTree {
    /// # Errors
    ///
    /// A [`TreeError`] when the walk itself cannot proceed. A tree that is
    /// present but damaged is a populated [`TreeReport`], not an error.
    fn verify(&self, artifact: &str) -> Result<TreeReport, TreeError>;
}

/// Wall-clock reads and the waiter's sleep.
///
/// Injected because three behaviours turn on time — the reaper's age backstop,
/// the single-flight waiter's bound and the executor's sticky-marker TTL — and
/// without a seam each could only be tested by sleeping or by back-dating
/// mtimes.
pub trait Clock {
    /// Seconds since the Unix epoch, for comparing against a recorded mtime.
    fn now_seconds(&self) -> u64;
    /// Sleeps for the waiter's poll interval.
    fn sleep_poll_interval(&self);
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::ffi::OsString;
    use std::time::Duration;

    use crate::launch::core::{swallow_under_fail_safe, ResolutionError};

    use super::{Discrepancy, ErrorClass, TreeError, TreeReport};

    /// One of every variant. `class()`'s own match is wildcard-free, so a new
    /// variant cannot ship unclassified; this list is what keeps the
    /// classification itself asserted rather than merely present.
    fn one_of_each() -> Vec<(TreeError, ErrorClass)> {
        vec![
            (
                TreeError::Attestation {
                    detail: "signature does not verify".to_owned(),
                },
                ErrorClass::Refusal,
            ),
            (
                TreeError::UnexpectedDigest {
                    artifact: "browser".to_owned(),
                    expected: "a".repeat(64),
                    found: "b".repeat(64),
                },
                ErrorClass::Refusal,
            ),
            (
                TreeError::PathEscape {
                    entry: "../escape".to_owned(),
                },
                ErrorClass::Refusal,
            ),
            (
                TreeError::Extraction {
                    detail: "a member disagrees with the table".to_owned(),
                },
                ErrorClass::Refusal,
            ),
            (
                TreeError::Seal {
                    detail: "chmod refused".to_owned(),
                },
                ErrorClass::Refusal,
            ),
            (TreeError::TableMissing, ErrorClass::Refusal),
            (
                TreeError::LayoutUnsupported {
                    found: 2,
                    supported: 1,
                },
                ErrorClass::Failed,
            ),
            (
                TreeError::Pointer {
                    detail: "unparseable".to_owned(),
                },
                ErrorClass::Failed,
            ),
            (
                TreeError::Lease {
                    detail: "ENOLCK".to_owned(),
                },
                ErrorClass::Failed,
            ),
            (
                TreeError::DiskShortfall {
                    needed: 600,
                    available: 10,
                },
                ErrorClass::Failed,
            ),
            (
                TreeError::MaterialisationInProgress {
                    waited: Duration::from_secs(5),
                },
                ErrorClass::Failed,
            ),
        ]
    }

    #[test]
    fn every_variant_carries_its_stated_class() {
        for (error, expected) in one_of_each() {
            assert_eq!(error.class(), expected, "{error} was misclassified");
        }
    }

    #[test]
    fn a_hostile_archive_is_a_refusal_and_local_damage_is_not() {
        assert_eq!(
            TreeError::PathEscape {
                entry: "../escape".to_owned()
            }
            .class(),
            ErrorClass::Refusal
        );
        assert_eq!(
            TreeError::Pointer {
                detail: "unparseable".to_owned()
            }
            .class(),
            ErrorClass::Failed
        );
    }

    #[test]
    fn the_class_decides_whether_fail_safe_swallows_the_failure() {
        let args = vec![OsString::from("--fail-safe")];
        for (error, class) in one_of_each() {
            let kernel_error =
                kernel::Error::from(ResolutionError::Tree(error));
            let swallowed = swallow_under_fail_safe(&kernel_error, &args);
            assert_eq!(
                swallowed,
                class == ErrorClass::Failed,
                "{kernel_error} was swallowed against its class"
            );
        }
    }

    #[test]
    fn a_report_with_no_findings_is_the_sound_one() {
        let sound = TreeReport {
            artifact: "browser".to_owned(),
            findings: Vec::new(),
        };
        assert!(sound.is_sound());
        let damaged = TreeReport {
            artifact: "browser".to_owned(),
            findings: vec![("lib/icudtl.dat".to_owned(), Discrepancy::Digest)],
        };
        assert!(!damaged.is_sound());
    }
}
