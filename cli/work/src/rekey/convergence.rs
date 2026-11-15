//! Whether a corpus already holds every work item under the ID the
//! configured scheme gives it, so commands may read and write IDs without
//! first re-keying.

use std::fmt::Display;
use std::fmt::Formatter;
use std::path::Path;
use std::path::PathBuf;

use crate::draft_id::DraftId;
use crate::id_pattern::escaped;
use crate::id_scheme::IdOwnership;
use crate::id_scheme::IdScheme;
use crate::identity::ItemIdentity;
use crate::rekey::mapping::why_unclassifiable;
use crate::rekey::outcome::unclassifiable;
use crate::rekey::outcome::unclassifiable_remedy;
use crate::rekey::outcome::UnclassifiableCause;
use crate::shown_path::relative_to;
use crate::tracker_key::TrackerKey;

/// Whether `identity` already holds an ID the scheme gives.
///
/// Under a numeric pattern, that is an `id` other than a draft's in the
/// pattern's shape under any key, whatever its case, since `--project`
/// writes items under keys other than `work.key`; under `{tracker}`, a
/// draft or an item named by its issue.
#[must_use]
pub fn is_converged(identity: &ItemIdentity, scheme: &IdScheme) -> bool {
    match scheme.ownership() {
        IdOwnership::Local => {
            DraftId::parse(&identity.id).is_none()
                && scheme.is_in_shape(&identity.id)
        }
        IdOwnership::Tracker => {
            DraftId::parse(&identity.id).is_some()
                || identity.external_id.as_deref().is_some_and(|key| {
                    TrackerKey::parse_under(key, scheme.tracker_keys())
                        .is_some()
                        && key.eq_ignore_ascii_case(&identity.id)
                })
        }
    }
}

/// An item re-key would move, by its path and the `id` it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OffPatternItem {
    pub path: PathBuf,
    pub id: String,
}

/// An off-pattern item whose IDs re-key cannot read, which must be fixed by
/// hand before re-key can converge the rest.
#[derive(Debug, Clone, PartialEq, Eq)]
struct UnclassifiableItem {
    path: PathBuf,
    cause: UnclassifiableCause,
}

/// A corpus holding items re-key would move, which commands that read or
/// write IDs refuse to run over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RekeyRequired {
    pattern: String,
    off_pattern: Vec<OffPatternItem>,
    unclassifiable: Option<UnclassifiableItem>,
}

impl RekeyRequired {
    pub const CODE: &'static str = "E_REKEY_REQUIRED";

    #[must_use]
    pub const fn code(&self) -> &'static str {
        Self::CODE
    }

    #[must_use]
    pub fn off_pattern(&self) -> &[OffPatternItem] {
        &self.off_pattern
    }

    /// The refusal with every path relative to `root`.
    #[must_use]
    pub fn relative_to(self, root: &Path) -> Self {
        Self {
            off_pattern: self
                .off_pattern
                .into_iter()
                .map(|item| OffPatternItem {
                    path: relative_to(item.path, root),
                    id: item.id,
                })
                .collect(),
            unclassifiable: self.unclassifiable.map(|item| {
                UnclassifiableItem {
                    path: relative_to(item.path, root),
                    cause: item.cause,
                }
            }),
            ..self
        }
    }
}

impl Display for RekeyRequired {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let count = self.off_pattern.len();
        let (noun, verb) = if count == 1 {
            ("work item", "is")
        } else {
            ("work items", "are")
        };
        let first = &self.off_pattern[0];
        write!(
            formatter,
            "{}: {count} {noun} {verb} not under the ID `work.id_pattern` \
             '{}' gives them, such as {} with `id` '{}'. ",
            Self::CODE,
            escaped(&self.pattern),
            escaped(&first.path.to_string_lossy()),
            escaped(&first.id),
        )?;
        let then_run = match &self.unclassifiable {
            None => "Run",
            Some(item) => {
                write!(
                    formatter,
                    "Re-key cannot read one of them: {}. {} by hand, ",
                    unclassifiable(&item.path, &item.cause),
                    unclassifiable_remedy(&item.cause),
                )?;
                "then run"
            }
        };
        write!(
            formatter,
            "{then_run} `/rekey-work-items` (or `accelerator work rekey`) to \
             converge them, then re-run this command"
        )
    }
}

impl std::error::Error for RekeyRequired {}

/// Refuses a corpus holding any item re-key would move.
///
/// # Errors
///
/// [`RekeyRequired`] naming every off-pattern item, in path order.
pub fn refuse_divergence(
    scheme: &IdScheme,
    items: &[ItemIdentity],
) -> Result<(), RekeyRequired> {
    let mut off_pattern: Vec<OffPatternItem> = items
        .iter()
        .filter(|identity| !is_converged(identity, scheme))
        .map(|identity| OffPatternItem {
            path: identity.path.clone(),
            id: identity.id.clone(),
        })
        .collect();
    if off_pattern.is_empty() {
        return Ok(());
    }
    off_pattern.sort_by(|left, right| left.path.cmp(&right.path));
    let unclassifiable = items
        .iter()
        .filter(|identity| !is_converged(identity, scheme))
        .filter_map(|identity| {
            why_unclassifiable(identity, scheme).map(|cause| {
                UnclassifiableItem {
                    path: identity.path.clone(),
                    cause,
                }
            })
        })
        .min_by(|left, right| left.path.cmp(&right.path));
    Err(RekeyRequired {
        pattern: scheme.pattern().as_str().to_owned(),
        off_pattern,
        unclassifiable,
    })
}

#[cfg(test)]
// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use super::is_converged;
    use super::refuse_divergence;
    use crate::id_pattern::IdKey;
    use crate::id_pattern::IdPattern;
    use crate::id_scheme::IdScheme;
    use crate::identity::ItemIdentity;

    fn scheme(pattern: &str, key: Option<&str>) -> IdScheme {
        IdScheme::new(
            IdPattern::parse(pattern).unwrap_or_else(|_| unreachable!()),
            key.map(|key| IdKey::parse(key).unwrap_or_else(|_| unreachable!())),
        )
    }

    fn item(id: &str, external_id: Option<&str>) -> ItemIdentity {
        ItemIdentity {
            path: PathBuf::from(format!("/repo/meta/work/{id}-item.md")),
            id: id.to_owned(),
            aliases: Vec::new(),
            external_id: external_id.map(str::to_owned),
        }
    }

    #[test]
    fn an_id_in_the_target_shape_is_converged() {
        let keyed = scheme("{key}-{number:04d}", Some("PP"));
        for id in ["PP-0042", "pp-0042", "XY-0042"] {
            assert!(is_converged(&item(id, None), &keyed), "{id}");
        }
        assert!(is_converged(
            &item("0042", None),
            &scheme("{number:04d}", None)
        ));
        assert!(is_converged(
            &item("v001", None),
            &scheme("v{number:03d}", None)
        ));
    }

    #[test]
    fn an_id_in_shape_is_converged_whatever_its_literal_case() {
        for (id, pattern, key) in [
            ("V001", "v{number:03d}", None),
            ("pp-0042", "PP-{number:04d}", None),
            ("PPV001", "{key}v{number:03d}", Some("PP")),
        ] {
            assert!(
                is_converged(&item(id, None), &scheme(pattern, key)),
                "{id}"
            );
        }
    }

    #[test]
    fn an_id_in_shape_matching_its_external_id_is_converged() {
        let aligned = scheme("{key}-{number:04d}", Some("ENG"));

        assert!(is_converged(&item("ENG-1234", Some("ENG-1234")), &aligned));
    }

    #[test]
    fn an_id_re_key_would_move_is_not_converged() {
        let keyed = scheme("{key}-{number:04d}", Some("PP"));
        for (id, external_id) in [
            ("0042", None),
            ("PP-42", None),
            ("PP_0042", None),
            ("ENG-7", Some("ENG-7")),
            ("draft-k7mq3x", None),
            ("notes", None),
        ] {
            assert!(!is_converged(&item(id, external_id), &keyed), "{id}");
        }
        assert!(!is_converged(
            &item("0042", None),
            &scheme("{number:05d}", None)
        ));
    }

    #[test]
    fn under_tracker_only_drafts_and_items_named_by_their_issue_converge() {
        let tracker = scheme("{tracker}", None);
        assert!(is_converged(&item("draft-k7mq3x", None), &tracker));
        assert!(is_converged(&item("ENG-7", Some("eng-7")), &tracker));
        assert!(!is_converged(&item("0042", None), &tracker));
        assert!(!is_converged(&item("0042", Some("ENG-7")), &tracker));
        assert!(!is_converged(&item("ENG-7", Some("not a key")), &tracker));
        let linear = tracker.with_tracker_keys(tracker::TrackerKeys::Linear);
        assert!(!is_converged(
            &item("MY_PROJ-7", Some("MY_PROJ-7")),
            &linear
        ));
    }

    #[test]
    fn a_converged_corpus_is_not_refused() {
        let keyed = scheme("{key}-{number:04d}", Some("PP"));
        let items = [item("PP-0001", None), item("PP-0002", None)];
        assert_eq!(refuse_divergence(&keyed, &items), Ok(()));
    }

    #[test]
    fn a_divergent_corpus_is_refused_naming_its_first_off_pattern_item() {
        let keyed = scheme("{key}_{number:04d}", Some("PP"));
        let items = [
            item("PP_0001", None),
            item("PP-0003", None),
            item("PP-0002", None),
        ];

        let refusal = refuse_divergence(&keyed, &items)
            .err()
            .unwrap_or_else(|| unreachable!("the corpus diverges"))
            .relative_to(Path::new("/repo"));

        assert_eq!(refusal.code(), "E_REKEY_REQUIRED");
        assert_eq!(
            refusal.to_string(),
            "E_REKEY_REQUIRED: 2 work items are not under the ID \
             `work.id_pattern` '{key}_{number:04d}' gives them, such as \
             meta/work/PP-0002-item.md with `id` 'PP-0002'. Run \
             `/rekey-work-items` (or `accelerator work rekey`) to converge \
             them, then re-run this command"
        );
    }

    #[test]
    fn an_item_re_key_cannot_read_is_named_with_its_manual_fix() {
        let keyed = scheme("{key}-{number:04d}", Some("PP"));
        let items = [item("PP-42", None), item("notes", None)];

        let refusal = refuse_divergence(&keyed, &items)
            .err()
            .unwrap_or_else(|| unreachable!("the corpus diverges"))
            .relative_to(Path::new("/repo"));

        assert_eq!(
            refusal.to_string(),
            "E_REKEY_REQUIRED: 2 work items are not under the ID \
             `work.id_pattern` '{key}-{number:04d}' gives them, such as \
             meta/work/PP-42-item.md with `id` 'PP-42'. Re-key cannot read \
             one of them: meta/work/notes-item.md has `id` 'notes', which \
             holds no number. Give the item an `id` in the shape of \
             `work.id_pattern` by hand, then run `/rekey-work-items` (or \
             `accelerator work rekey`) to converge them, then re-run this \
             command"
        );
    }

    #[test]
    fn an_external_id_re_key_cannot_read_is_named_with_its_manual_fix() {
        let refusal = refuse_divergence(
            &scheme("{tracker}", None),
            &[item("0042", Some("not a key"))],
        )
        .err()
        .unwrap_or_else(|| unreachable!("the corpus diverges"))
        .relative_to(Path::new("/repo"));

        assert!(
            refusal.to_string().contains(
                "Re-key cannot read one of them: meta/work/0042-item.md has \
                 `external_id` 'not a key', which is not shaped like a \
                 tracker key. Correct the item's `external_id` to the key of \
                 its tracker issue by hand, then run `/rekey-work-items`"
            ),
            "{refusal}"
        );
    }

    #[test]
    fn a_hostile_id_in_the_refusal_is_escaped() {
        let refusal = refuse_divergence(
            &scheme("{number:04d}", None),
            &[item("x\u{1b}[H", None)],
        )
        .err()
        .unwrap_or_else(|| unreachable!("the corpus diverges"))
        .to_string();

        assert!(!refusal.contains('\u{1b}'), "{refusal:?}");
        assert!(refusal.contains("1 work item is not"), "{refusal}");
    }
}
