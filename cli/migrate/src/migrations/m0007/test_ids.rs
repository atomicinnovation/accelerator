use corpus::LinkageIds;

/// The legacy rules: a filename's leading digits, and exactly four digits in
/// body text.
pub struct LegacyDigits;

impl LinkageIds for LegacyDigits {
    fn id_in_filename(&self, filename: &str) -> Option<String> {
        let digits: String =
            filename.chars().take_while(char::is_ascii_digit).collect();
        (!digits.is_empty()).then_some(digits)
    }

    fn is_bare_id(&self, token: &str) -> bool {
        token.len() == 4 && token.bytes().all(|byte| byte.is_ascii_digit())
    }
}
