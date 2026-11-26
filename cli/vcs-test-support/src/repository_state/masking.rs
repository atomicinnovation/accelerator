//! Replacing the commit and change IDs a file names with the positions of the
//! commits they name, so a file recording an ID compares by graph position.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Captures;
use regex::Regex;

use super::CommitPosition;
use super::RawCommit;

/// Every full commit ID and change ID a repository holds, each with the text
/// that stands in for it.
///
/// Abbreviated IDs are deliberately left alone: a file holding one makes two
/// builds compare unequal, which fails loudly rather than passing.
///
/// A `+`-joined list comes out ordered by position. Its writer orders it by
/// hash, so two builds of one merge would otherwise list it differently.
///
/// An ID of a commit `positions` does not yet hold stands in as one with no
/// position, so a masking made while positions are still being given out
/// never lets an ID decide anything.
#[derive(Debug)]
pub(super) struct IdMasking {
    replacements: Vec<(String, String)>,
}

impl IdMasking {
    pub(super) fn of(
        commits: &[RawCommit],
        positions: &BTreeMap<String, CommitPosition>,
    ) -> Self {
        let mut replacements = Vec::new();
        for commit in commits {
            let position = positions
                .get(&commit.id)
                .map(|CommitPosition(position)| format!(" {position}"))
                .unwrap_or_default();
            replacements
                .push((commit.id.clone(), format!("<commit{position}>")));
            if let Some(change) = &commit.change {
                replacements
                    .push((change.clone(), format!("<change{position}>")));
            }
        }
        Self { replacements }
    }

    pub(super) fn mask(&self, text: &str) -> String {
        let masked = self
            .replacements
            .iter()
            .fold(text.to_owned(), |masked, (id, stand_in)| {
                masked.replace(id.as_str(), stand_in)
            });
        JOINED_STAND_INS
            .replace_all(&masked, |list: &Captures<'_>| {
                let mut stand_ins: Vec<&str> = list[0].split('+').collect();
                stand_ins.sort_by_key(|stand_in| StandIn::parse(stand_in));
                stand_ins.join("+")
            })
            .into_owned()
    }
}

static JOINED_STAND_INS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<(?:commit|change) \d+>(?:\+<(?:commit|change) \d+>)+")
        .unwrap_or_else(|_| unreachable!("the pattern is a literal"))
});

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct StandIn<'s> {
    position: Option<usize>,
    kind: &'s str,
}

impl<'s> StandIn<'s> {
    fn parse(text: &'s str) -> Self {
        let inner = text.trim_start_matches('<').trim_end_matches('>');
        let (kind, position) = inner.split_once(' ').unwrap_or((inner, ""));
        Self {
            position: position.parse().ok(),
            kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::IdMasking;
    use crate::repository_state::CommitPosition;
    use crate::repository_state::RawCommit;

    const FIRST_ID: &str = "3c04a98ce9421ac809a213552b91d5efa8b1e405";
    const FIRST_CHANGE: &str = "lpsymouyvmrswpkpulvuwnqwpkoulsrk";
    const SECOND_ID: &str = "7f069366d9538cd1c901539b05b35ae3adc97b8c";
    const SECOND_CHANGE: &str = "pppwmltotsvusorlpzwtrsykrwwpzywz";

    fn masking() -> IdMasking {
        let commit = |id: &str, change: &str| RawCommit {
            id: id.to_owned(),
            change: Some(change.to_owned()),
            parents: Vec::new(),
            description: String::new(),
            files: BTreeMap::new(),
        };
        IdMasking::of(
            &[
                commit(FIRST_ID, FIRST_CHANGE),
                commit(SECOND_ID, SECOND_CHANGE),
            ],
            &BTreeMap::from([
                (FIRST_ID.to_owned(), CommitPosition(0)),
                (SECOND_ID.to_owned(), CommitPosition(1)),
            ]),
        )
    }

    #[test]
    fn a_commit_id_alone_becomes_its_position() {
        assert_eq!(masking().mask(&format!("{SECOND_ID}\n")), "<commit 1>\n");
    }

    #[test]
    fn a_change_id_becomes_its_commit_s_position() {
        assert_eq!(masking().mask(FIRST_CHANGE), "<change 0>");
    }

    #[test]
    fn each_id_in_a_joined_list_becomes_its_position() {
        assert_eq!(
            masking().mask(&format!("{FIRST_ID}+{SECOND_ID}")),
            "<commit 0>+<commit 1>"
        );
    }

    #[test]
    fn a_joined_list_is_ordered_by_position_whatever_order_names_it() {
        assert_eq!(
            masking().mask(&format!("{SECOND_ID}+{FIRST_ID}\n")),
            "<commit 0>+<commit 1>\n"
        );
    }

    #[test]
    fn an_abbreviated_id_is_left_alone() {
        assert_eq!(masking().mask(&FIRST_ID[..12]), &FIRST_ID[..12]);
    }

    #[test]
    fn text_naming_no_id_is_unchanged() {
        assert_eq!(masking().mask("nothing here"), "nothing here");
    }
}
