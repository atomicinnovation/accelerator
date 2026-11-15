//! The work-item ID reads typed linkage makes.

/// How linkage reads a work-item ID from a path and from body text.
pub trait LinkageIds {
    fn id_in_filename(&self, filename: &str) -> Option<String>;

    /// The bare work-item IDs a `## Dependencies` line names, in order.
    fn bare_ids_in(&self, line: &str) -> Vec<String>;
}

#[cfg(test)]
pub(crate) mod test_ids {
    use super::LinkageIds;

    /// The legacy rules: a filename's leading digits, and exactly four
    /// digits in body text.
    pub struct LegacyDigits;

    impl LinkageIds for LegacyDigits {
        fn id_in_filename(&self, filename: &str) -> Option<String> {
            let digits: String =
                filename.chars().take_while(char::is_ascii_digit).collect();
            (!digits.is_empty()).then_some(digits)
        }

        fn bare_ids_in(&self, line: &str) -> Vec<String> {
            let characters: Vec<char> = line.chars().collect();
            let mut tokens = Vec::new();
            let mut index = 0;
            while index + 4 <= characters.len() {
                let candidate: String =
                    characters[index..index + 4].iter().collect();
                if candidate.bytes().all(|byte| byte.is_ascii_digit()) {
                    tokens.push(candidate);
                    index += 4;
                } else {
                    index += 1;
                }
            }
            tokens
        }
    }
}
