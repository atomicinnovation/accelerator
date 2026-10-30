//! Names the directory a pull keeps an item's edits in, under the
//! repository's id scheme.

use std::cell::RefCell;

use corpus::WorkItemIdScheme;
use work::draft_id::is_token_safe;
use work::draft_id::SuffixDraws;
use work::draft_id::ALPHABET;
use work_adapters::sync::recovery_label::RecoveryLabel;
use work_adapters::sync::recovery_label::RecoveryLabels;

pub struct DrawnRecoveryLabels<'a, D> {
    scheme: &'a WorkItemIdScheme,
    draws: RefCell<D>,
}

impl<'a, D: SuffixDraws> DrawnRecoveryLabels<'a, D> {
    pub const fn new(scheme: &'a WorkItemIdScheme, draws: D) -> Self {
        Self {
            scheme,
            draws: RefCell::new(draws),
        }
    }
}

impl<D: SuffixDraws> RecoveryLabels for DrawnRecoveryLabels<'_, D> {
    fn label_for(&self, id: &str, epoch: u64) -> Option<RecoveryLabel> {
        if !is_token_safe(self.scheme, id) {
            return None;
        }
        let suffix: String = self
            .draws
            .borrow_mut()
            .draw()
            .iter()
            .map(|&index| {
                char::from(ALPHABET[usize::from(index) % ALPHABET.len()])
            })
            .collect();
        Some(RecoveryLabel::new(epoch, &suffix, id))
    }
}

#[cfg(test)]
mod tests {
    use corpus::WorkItemIdScheme;
    use work::draft_id::SuffixDraws;
    use work_adapters::draft_id::RandomSuffixDraws;
    use work_adapters::sync::recovery_label::RecoveryLabel;
    use work_adapters::sync::recovery_label::RecoveryLabels as _;

    use super::DrawnRecoveryLabels;

    struct Fixed([u8; 6]);

    impl SuffixDraws for Fixed {
        fn draw(&mut self) -> [u8; 6] {
            self.0
        }
    }

    #[test]
    fn an_id_unsafe_for_a_directory_gets_no_label() {
        let scheme = WorkItemIdScheme::numeric();
        let labels = DrawnRecoveryLabels::new(&scheme, RandomSuffixDraws);

        assert_eq!(labels.label_for("../0001", 1_700_000_000), None);
    }

    #[test]
    fn a_label_carries_a_suffix_drawn_over_the_draft_id_alphabet() {
        let scheme = WorkItemIdScheme::numeric();
        let labels =
            DrawnRecoveryLabels::new(&scheme, Fixed([20, 7, 21, 31, 3, 32]));

        assert_eq!(
            labels.label_for("0001", 1_700_000_000),
            Some(RecoveryLabel::new(1_700_000_000, "m7nz30", "0001"))
        );
    }
}
