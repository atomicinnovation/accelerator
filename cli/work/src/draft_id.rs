//! The provisional identity of a work item no tracker has confirmed yet.

use crate::identity::holder_of;
use crate::identity::ItemIdentity;

const SUFFIX_LENGTH: usize = 6;

/// The lowercase Crockford base-32 alphabet a draft suffix is drawn from.
pub const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

const MINT_ATTEMPTS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftId(String);

impl DraftId {
    pub const PREFIX: &'static str = "draft-";

    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        corpus::canonical_draft_id(raw).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A source of random draft suffixes.
pub trait SuffixDraws {
    /// Six indices into [`ALPHABET`]; an index past its end wraps.
    fn draw(&mut self) -> [u8; SUFFIX_LENGTH];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DraftIdExhausted {
    pub attempts: usize,
}

impl std::fmt::Display for DraftIdExhausted {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "E_DRAFT_ID_EXHAUSTED: no free draft ID after {} draws",
            self.attempts
        )
    }
}

impl std::error::Error for DraftIdExhausted {}

/// Draws until a suffix with a letter names no item's `id` or `aliases`.
///
/// # Errors
///
/// [`DraftIdExhausted`] when every one of the bounded draws was rejected.
pub fn mint_draft_id(
    draws: &mut dyn SuffixDraws,
    items: &[ItemIdentity],
) -> Result<DraftId, DraftIdExhausted> {
    (0..MINT_ATTEMPTS)
        .filter_map(|_| {
            let suffix: String = draws
                .draw()
                .iter()
                .map(|&index| {
                    char::from(ALPHABET[usize::from(index) % ALPHABET.len()])
                })
                .collect();
            DraftId::parse(&format!("{}{suffix}", DraftId::PREFIX))
        })
        .find(|candidate| holder_of(candidate.as_str(), items).is_none())
        .ok_or(DraftIdExhausted {
            attempts: MINT_ATTEMPTS,
        })
}

/// `count` draft IDs distinct from each other and from every item's `id` and
/// `aliases`.
///
/// # Errors
///
/// [`DraftIdExhausted`] when any one ID exhausts its draws.
pub fn mint_draft_ids(
    draws: &mut dyn SuffixDraws,
    items: &[ItemIdentity],
    count: usize,
) -> Result<Vec<DraftId>, DraftIdExhausted> {
    let mut held = items.to_vec();
    let mut minted = Vec::with_capacity(count);
    for _ in 0..count {
        let draft = mint_draft_id(draws, &held)?;
        held.push(ItemIdentity {
            path: std::path::PathBuf::new(),
            id: draft.as_str().to_owned(),
            aliases: Vec::new(),
            external_id: None,
        });
        minted.push(draft);
    }
    Ok(minted)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::path::PathBuf;

    use super::mint_draft_id;
    use super::mint_draft_ids;
    use super::DraftId;
    use super::DraftIdExhausted;
    use super::SuffixDraws;
    use super::ALPHABET;
    use crate::identity::ItemIdentity;

    struct ScriptedDraws {
        script: VecDeque<[u8; 6]>,
        drawn: usize,
    }

    impl ScriptedDraws {
        fn of(suffixes: &[&str]) -> Self {
            let script = suffixes
                .iter()
                .map(|suffix| {
                    let mut draw = [0u8; 6];
                    for (slot, c) in draw.iter_mut().zip(suffix.bytes()) {
                        *slot = u8::try_from(
                            ALPHABET.iter().position(|&a| a == c).unwrap_or(0),
                        )
                        .unwrap_or(0);
                    }
                    draw
                })
                .collect();
            Self { script, drawn: 0 }
        }
    }

    impl SuffixDraws for ScriptedDraws {
        fn draw(&mut self) -> [u8; 6] {
            self.drawn += 1;
            self.script.pop_front().unwrap_or([0; 6])
        }
    }

    fn holding(id: &str, aliases: &[&str]) -> ItemIdentity {
        ItemIdentity {
            path: PathBuf::from(format!("meta/work/{id}-title.md")),
            id: id.to_owned(),
            aliases: aliases.iter().map(|&alias| alias.to_owned()).collect(),
            external_id: None,
        }
    }

    fn minted(
        draws: &mut ScriptedDraws,
        items: &[ItemIdentity],
    ) -> Option<String> {
        mint_draft_id(draws, items)
            .ok()
            .map(|id| id.as_str().to_owned())
    }

    #[test]
    fn an_all_digit_draw_is_redrawn() {
        let mut draws = ScriptedDraws::of(&["123456", "k7mq3x"]);

        assert_eq!(minted(&mut draws, &[]).as_deref(), Some("draft-k7mq3x"));
        assert_eq!(draws.drawn, 2);
    }

    #[test]
    fn a_draw_colliding_with_an_alias_is_redrawn() {
        let items = [holding("ENG-1", &["draft-k7mq3x"])];
        let mut draws = ScriptedDraws::of(&["k7mq3x", "a1b2c3"]);

        assert_eq!(minted(&mut draws, &items).as_deref(), Some("draft-a1b2c3"));
    }

    #[test]
    fn a_draw_colliding_with_an_existing_draft_is_redrawn() {
        let items = [holding("draft-k7mq3x", &[])];
        let mut draws = ScriptedDraws::of(&["k7mq3x", "a1b2c3"]);

        assert_eq!(minted(&mut draws, &items).as_deref(), Some("draft-a1b2c3"));
    }

    #[test]
    fn fifteen_collisions_then_a_free_draw_succeeds() {
        let items = [holding("draft-k7mq3x", &[])];
        let mut script = vec!["k7mq3x"; 15];
        script.push("a1b2c3");
        let mut draws = ScriptedDraws::of(&script);

        assert_eq!(minted(&mut draws, &items).as_deref(), Some("draft-a1b2c3"));
        assert_eq!(draws.drawn, 16);
    }

    #[test]
    fn sixteen_collisions_exhaust_after_exactly_sixteen_draws() {
        let items = [holding("draft-k7mq3x", &[])];
        let mut draws = ScriptedDraws::of(&["k7mq3x"; 17]);

        assert_eq!(
            mint_draft_id(&mut draws, &items),
            Err(DraftIdExhausted { attempts: 16 })
        );
        assert_eq!(draws.drawn, 16);
    }

    #[test]
    fn a_batch_never_mints_the_same_draft_id_twice() {
        let mut draws = ScriptedDraws::of(&["k7mq3x", "k7mq3x", "a1b2c3"]);

        let batch = mint_draft_ids(&mut draws, &[], 2)
            .map(|ids| ids.iter().map(|id| id.as_str().to_owned()).collect());

        assert_eq!(
            batch,
            Ok(vec!["draft-k7mq3x".to_owned(), "draft-a1b2c3".to_owned()])
        );
    }

    #[test]
    fn every_minted_suffix_uses_only_lowercase_crockford_characters() {
        let every_index: Vec<[u8; 6]> = (0u8..32)
            .map(|index| [index, index, index, index, index, 10])
            .collect();
        let mut draws = ScriptedDraws {
            script: every_index.into(),
            drawn: 0,
        };

        for _ in 0..32 {
            let id = minted(&mut draws, &[]).unwrap_or_default();
            let suffix = id.strip_prefix(DraftId::PREFIX).unwrap_or_default();
            assert!(
                suffix.len() == 6
                    && suffix
                        .chars()
                        .all(|c| "0123456789abcdefghjkmnpqrstvwxyz".contains(c)),
                "{id}"
            );
        }
    }

    #[test]
    fn a_draft_id_has_the_draft_prefix_and_six_crockford_characters() {
        assert_eq!(
            DraftId::parse("draft-k7mq3x").map(|id| id.as_str().to_owned()),
            Some("draft-k7mq3x".to_owned())
        );
        assert!(DraftId::parse("draft-k7mq3").is_none());
        assert!(DraftId::parse("draft-k7mq3xy").is_none());
        assert!(DraftId::parse("draft-k7mq3i").is_none());
        assert!(DraftId::parse("drafts-k7mq3x").is_none());
    }

    #[test]
    fn a_draft_id_is_read_case_insensitively_into_lowercase() {
        assert_eq!(
            DraftId::parse("DRAFT-K7MQ3X").map(|id| id.as_str().to_owned()),
            Some("draft-k7mq3x".to_owned())
        );
    }

    #[test]
    fn an_all_digit_draft_suffix_is_not_a_draft_id() {
        assert!(DraftId::parse("draft-123456").is_none());
    }
}
