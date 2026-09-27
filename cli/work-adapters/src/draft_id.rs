//! Random draft suffixes for `work::draft_id::mint_draft_id`.

use rand::Rng;
use work::draft_id::SuffixDraws;
use work::draft_id::ALPHABET;

pub struct RandomSuffixDraws;

impl SuffixDraws for RandomSuffixDraws {
    fn draw(&mut self) -> [u8; 6] {
        let mut rng = rand::rng();
        let bound = u8::try_from(ALPHABET.len()).unwrap_or(u8::MAX);
        std::array::from_fn(|_| rng.random_range(0..bound))
    }
}

#[cfg(test)]
mod tests {
    use work::draft_id::mint_draft_id;
    use work::draft_id::DraftId;

    use super::RandomSuffixDraws;

    #[test]
    fn random_draws_mint_well_formed_draft_ids() {
        for _ in 0..64 {
            let minted = mint_draft_id(&mut RandomSuffixDraws, &[])
                .map(|id| id.as_str().to_owned());

            assert!(
                minted
                    .as_deref()
                    .is_ok_and(|id| DraftId::parse(id).is_some()),
                "{minted:?}"
            );
        }
    }
}
