use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberPath<'a> {
    pub set_directory: &'a Path,
    pub set_relative_path: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaseOnlyRename<'a> {
    pub from: MemberPath<'a>,
    pub to: MemberPath<'a>,
}

impl MemberPath<'_> {
    fn differs_only_in_case_from(&self, other: &MemberPath<'_>) -> bool {
        self.set_directory == other.set_directory
            && self
                .set_relative_path
                .eq_ignore_ascii_case(other.set_relative_path)
    }
}

/// Pairs each removed member with an added member of the same set whose
/// path differs from it only in ASCII case: on a case-insensitive volume
/// that is one file renamed, not one removed and another added.
#[must_use]
pub fn pair_case_only_renames<'a>(
    removed: &[MemberPath<'a>],
    added: &[MemberPath<'a>],
) -> Vec<CaseOnlyRename<'a>> {
    let mut unpaired: Vec<&MemberPath<'a>> = added.iter().collect();
    removed
        .iter()
        .filter_map(|from| {
            let at = unpaired
                .iter()
                .position(|to| to.differs_only_in_case_from(from))?;
            Some(CaseOnlyRename {
                from: *from,
                to: *unpaired.remove(at),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{pair_case_only_renames, CaseOnlyRename, MemberPath};

    fn at<'a>(
        set_directory: &'a str,
        set_relative_path: &'a str,
    ) -> MemberPath<'a> {
        MemberPath {
            set_directory: Path::new(set_directory),
            set_relative_path,
        }
    }

    #[test]
    fn a_member_renamed_only_in_case_is_one_rename() {
        let removed = [at("inv", "screenshots/a.png")];
        let added = [at("inv", "screenshots/a.PNG")];

        assert_eq!(
            pair_case_only_renames(&removed, &added),
            [CaseOnlyRename {
                from: removed[0],
                to: added[0],
            }]
        );
    }

    #[test]
    fn members_of_different_sets_are_never_paired() {
        let removed = [at("one", "screenshots/a.png")];
        let added = [at("two", "screenshots/a.PNG")];

        assert!(pair_case_only_renames(&removed, &added).is_empty());
    }

    #[test]
    fn members_whose_names_differ_beyond_case_are_never_paired() {
        let removed = [at("inv", "screenshots/a.png")];
        let added = [at("inv", "screenshots/b.png")];

        assert!(pair_case_only_renames(&removed, &added).is_empty());
    }

    #[test]
    fn each_member_is_paired_at_most_once() {
        let removed = [at("inv", "screenshots/a.png")];
        let added = [
            at("inv", "screenshots/A.png"),
            at("inv", "screenshots/a.PNG"),
        ];

        assert_eq!(
            pair_case_only_renames(&removed, &added),
            [CaseOnlyRename {
                from: removed[0],
                to: added[0],
            }]
        );
    }

    #[test]
    fn every_case_only_rename_in_a_set_is_paired() {
        let removed = [at("inv", "a.png"), at("inv", "b.png")];
        let added = [at("inv", "B.png"), at("inv", "A.png")];

        assert_eq!(
            pair_case_only_renames(&removed, &added),
            [
                CaseOnlyRename {
                    from: removed[0],
                    to: added[1],
                },
                CaseOnlyRename {
                    from: removed[1],
                    to: added[0],
                },
            ]
        );
    }
}
