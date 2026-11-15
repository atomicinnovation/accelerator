//! IDs a tracker owns: its issue keys, and drafts until it confirms them.

use crate::draft_id::DraftId;
use crate::id_pattern::HeldPrefix;
use crate::id_pattern::IdKey;
use crate::id_pattern::IdParts;
use crate::id_pattern::IdPattern;
use crate::id_scheme::is_tracker_owned_token;
use crate::id_scheme::legacy_prefix;
use crate::id_scheme::CanonicaliseError;
use crate::id_scheme::FilenameIdentity;
use crate::id_scheme::IdReadings;
use crate::id_scheme::IdScheme;
use tracker::TrackerKeys;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TrackerIds {
    pattern: IdPattern,
    keys: TrackerKeys,
}

impl TrackerIds {
    pub(super) const fn new(pattern: IdPattern, keys: TrackerKeys) -> Self {
        Self { pattern, keys }
    }

    pub(super) const fn pattern(&self) -> &IdPattern {
        &self.pattern
    }
}

/// The tracker key leading `filename`, followed by the `-` that starts the
/// slug, with where the key ends.
fn tracker_key_prefix(
    filename: &str,
    keys: TrackerKeys,
) -> Option<(&str, &str, usize)> {
    let (key, rest) = filename.split_once('-')?;
    let digit_count = rest.bytes().take_while(u8::is_ascii_digit).count();
    let (digits, after) = rest.split_at(digit_count);
    let token = &filename[..=key.len() + digit_count];
    (keys.is_key(token) && after.starts_with('-')).then_some((
        key,
        digits,
        token.len(),
    ))
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
            .or_else(|| self.keys.is_key(trimmed).then(|| trimmed.to_owned()))
    }

    fn identity_key(&self, reference: &str) -> Option<String> {
        if let Some(draft) = DraftId::parse(reference) {
            return Some(draft.as_str().to_owned());
        }
        if self.keys.is_key(reference) {
            let (key, digits) = reference.rsplit_once('-')?;
            return Some(format!("{}-{digits}", key.to_ascii_uppercase()));
        }
        IdScheme::pad_legacy_number(reference)
    }

    fn filename_identity(&self, filename: &str) -> Option<FilenameIdentity> {
        if let Some(draft) = draft_prefix(filename) {
            return Some(FilenameIdentity {
                identity_key: draft.as_str().to_owned(),
                id_end: DraftId::LENGTH,
            });
        }
        let held = self.read_held_prefix(filename)?;
        Some(FilenameIdentity {
            identity_key: joined(held.parts())?,
            id_end: held.id_end(),
        })
    }

    fn read_held(&self, value: &str) -> Option<IdParts> {
        let (key, digits) = value.rsplit_once('-')?;
        self.keys
            .is_key(value)
            .then(|| IdParts::of(Some(key), digits))
            .flatten()
    }

    fn read_held_prefix(&self, filename: &str) -> Option<HeldPrefix> {
        if let Some((key, digits, id_end)) =
            tracker_key_prefix(filename, self.keys)
        {
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

    fn key(&self) -> Option<&IdKey> {
        None
    }

    fn is_bare_id(&self, token: &str) -> bool {
        is_tracker_owned_token(token, self.keys)
    }

    fn tracker_keys(&self) -> TrackerKeys {
        self.keys
    }
}
