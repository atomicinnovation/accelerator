//! IDs a tracker owns: its issue keys, and drafts until it confirms them.

use super::is_digits;
use super::CanonicaliseError;
use super::IdReadings;
use super::IdScheme;
use crate::draft_id::DraftId;
use crate::id_pattern::HeldPrefix;
use crate::id_pattern::IdParts;
use crate::id_pattern::IdPattern;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TrackerIds {
    pattern: IdPattern,
}

impl TrackerIds {
    pub(super) const fn new(pattern: IdPattern) -> Self {
        Self { pattern }
    }

    pub(super) const fn pattern(&self) -> &IdPattern {
        &self.pattern
    }
}

/// The tracker key leading `filename`, followed by the `-` that starts the
/// slug, with where the key ends.
fn tracker_key_prefix(filename: &str) -> Option<(&str, &str, usize)> {
    let (key, rest) = filename.split_once('-')?;
    let digit_count = rest.bytes().take_while(u8::is_ascii_digit).count();
    let (digits, after) = rest.split_at(digit_count);
    let token = &filename[..=key.len() + digit_count];
    (tracker::is_tracker_key(token) && after.starts_with('-')).then_some((
        key,
        digits,
        token.len(),
    ))
}

fn legacy_prefix(filename: &str) -> Option<&str> {
    let (digits, _) = filename.split_once('-')?;
    is_digits(digits).then_some(digits)
}

fn draft_prefix(filename: &str) -> Option<DraftId> {
    let candidate = filename.get(..DraftId::LENGTH)?;
    let followed_by_slug = filename[DraftId::LENGTH..].starts_with('-');
    DraftId::parse(candidate).filter(|_| followed_by_slug)
}

fn joined(parts: &IdParts) -> Option<String> {
    parts.key().map_or_else(
        || IdScheme::pad_legacy_number(parts.digits()),
        |key| Some(format!("{}-{}", key.to_ascii_uppercase(), parts.digits())),
    )
}

impl IdReadings for TrackerIds {
    fn displayed_id(&self, trimmed: &str) -> Option<String> {
        DraftId::parse(trimmed)
            .map(|draft| draft.as_str().to_owned())
            .or_else(|| {
                tracker::is_tracker_key(trimmed).then(|| trimmed.to_owned())
            })
    }

    fn identity_key(&self, reference: &str) -> Option<String> {
        if let Some(draft) = DraftId::parse(reference) {
            return Some(draft.as_str().to_owned());
        }
        if tracker::is_tracker_key(reference) {
            let (key, digits) = reference.rsplit_once('-')?;
            return Some(format!("{}-{digits}", key.to_ascii_uppercase()));
        }
        IdScheme::pad_legacy_number(reference)
    }

    fn filename_identity(&self, filename: &str) -> Option<(String, usize)> {
        if let Some(draft) = draft_prefix(filename) {
            return Some((draft.as_str().to_owned(), DraftId::LENGTH + 1));
        }
        let held = self.read_held_prefix(filename)?;
        Some((joined(held.parts())?, held.slug_start()))
    }

    fn read_held(&self, value: &str) -> Option<IdParts> {
        let (key, digits) = value.rsplit_once('-')?;
        tracker::is_tracker_key(value)
            .then(|| IdParts::of(Some(key), digits))
            .flatten()
    }

    fn read_held_prefix(&self, filename: &str) -> Option<HeldPrefix> {
        if let Some((key, digits, id_end)) = tracker_key_prefix(filename) {
            return Some(HeldPrefix::new(
                IdParts::of(Some(key), digits)?,
                id_end,
            ));
        }
        let digits = legacy_prefix(filename)?;
        Some(HeldPrefix::new(IdParts::of(None, digits)?, digits.len()))
    }

    fn holds(&self, held: &str, id: &IdParts) -> bool {
        self.read_held(held).is_some_and(|reading| {
            reading.same_id(id) && reading.digits() == id.digits()
        })
    }

    fn canonicalise(&self, input: &str) -> Result<String, CanonicaliseError> {
        Ok(input.to_owned())
    }
}
