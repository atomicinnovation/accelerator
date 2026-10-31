//! The ID convention `slug` and `cluster` read work-item IDs through.

/// How a corpus spells the IDs of its work items. Every operation gives the
/// same [`Self::identity_key`] for one item, so a filename, a `work-item:`
/// reference and a frontmatter `id` naming it all join.
pub trait IdConvention {
    /// The byte offset just past the ID prefix and its separator.
    fn slug_start(&self, stem: &str) -> Option<usize>;

    fn is_id(&self, token: &str) -> bool;

    fn identity_key(&self, reference: &str) -> Option<String>;

    fn id_in_filename(&self, filename: &str) -> Option<String>;
}

#[cfg(test)]
pub(crate) mod test_ids {
    use super::IdConvention;

    /// Four-digit IDs, under `KEY-` when a key is set.
    pub struct FourDigitIds {
        pub key: Option<&'static str>,
    }

    pub const NUMERIC: FourDigitIds = FourDigitIds { key: None };

    impl FourDigitIds {
        fn after_key<'a>(&self, text: &'a str) -> Option<&'a str> {
            match self.key {
                Some(key) => text.strip_prefix(key)?.strip_prefix('-'),
                None => Some(text),
            }
        }

        fn keyed(&self, digits: &str) -> String {
            self.key.map_or_else(
                || digits.to_owned(),
                |key| format!("{key}-{digits}"),
            )
        }
    }

    fn leading_digits(text: &str) -> &str {
        let end = text
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(text.len());
        &text[..end]
    }

    impl IdConvention for FourDigitIds {
        fn slug_start(&self, stem: &str) -> Option<usize> {
            let rest = self.after_key(stem)?;
            let digits = leading_digits(rest);
            let separated =
                !digits.is_empty() && rest[digits.len()..].starts_with('-');
            separated.then(|| stem.len() - rest.len() + digits.len() + 1)
        }

        fn is_id(&self, token: &str) -> bool {
            self.after_key(token).is_some_and(|digits| {
                digits.len() == 4 && digits.chars().all(|c| c.is_ascii_digit())
            })
        }

        fn identity_key(&self, reference: &str) -> Option<String> {
            let number: u64 = reference.parse().ok()?;
            Some(self.keyed(&format!("{number:04}")))
        }

        fn id_in_filename(&self, filename: &str) -> Option<String> {
            let digits = leading_digits(filename);
            let separated =
                !digits.is_empty() && filename[digits.len()..].starts_with('-');
            separated.then(|| self.keyed(digits))
        }
    }
}
