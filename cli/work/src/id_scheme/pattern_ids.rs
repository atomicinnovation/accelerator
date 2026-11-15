//! IDs the repository owns, rendered from its pattern and key.

use crate::id_pattern::HeldPrefix;
use crate::id_pattern::IdKey;
use crate::id_pattern::IdParts;
use crate::id_pattern::IdPattern;
use crate::id_pattern::MissingKey;
use crate::id_scheme::is_digits;
use crate::id_scheme::legacy_prefix;
use crate::id_scheme::CanonicaliseError;
use crate::id_scheme::FilenameIdentity;
use crate::id_scheme::IdReadings;
use crate::id_scheme::LEGACY_WIDTH;
use tracker::TrackerKeys;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PatternIds {
    pattern: IdPattern,
    key: Option<IdKey>,
    tracker_keys: TrackerKeys,
}

impl PatternIds {
    /// The IDs of `pattern`, holding `key` only when the pattern has a key
    /// token, beside `external_id`s read as tracker keys by `tracker_keys`.
    pub(super) fn new(
        pattern: IdPattern,
        key: Option<IdKey>,
        tracker_keys: TrackerKeys,
    ) -> Self {
        let key = key.filter(|_| pattern.references_key());
        Self {
            pattern,
            key,
            tracker_keys,
        }
    }

    pub(super) const fn pattern(&self) -> &IdPattern {
        &self.pattern
    }

    fn padded(&self, number: u64) -> String {
        let width = self.pattern.width().unwrap_or(LEGACY_WIDTH);
        format!("{number:0width$}")
    }

    /// `key` in the spelling a join key uses: `work.key`'s when it names the
    /// same key, otherwise upper case.
    fn folded(&self, key: &str) -> String {
        match &self.key {
            Some(own) if own.as_str().eq_ignore_ascii_case(key) => {
                own.as_str().to_owned()
            }
            _ => key.to_ascii_uppercase(),
        }
    }

    fn joined(&self, parts: &IdParts) -> Option<String> {
        let key = parts
            .key()
            .map(|key| IdKey::parse(&self.folded(key)))
            .transpose()
            .ok()?;
        self.pattern.render(key.as_ref(), parts.number()).ok()
    }

    fn render_keeping_key(
        &self,
        parts: &IdParts,
    ) -> Result<String, MissingKey> {
        let key = parts.key().and_then(|key| IdKey::parse(key).ok());
        self.pattern.render(key.as_ref(), parts.number())
    }

    /// A bare number read as the pattern's number: under `work.key` when
    /// the pattern has a key token, which then needs one.
    fn bare_number(&self, digits: &str) -> Option<IdParts> {
        if !is_digits(digits) {
            return None;
        }
        if self.pattern.references_key() {
            IdParts::of(Some(self.key.as_ref()?.as_str()), digits)
        } else {
            IdParts::of(None, digits)
        }
    }
}

impl IdReadings for PatternIds {
    fn displayed_id(&self, trimmed: &str) -> Option<String> {
        if IdParts::parse(&self.pattern, trimmed).is_some() {
            return Some(trimmed.to_owned());
        }
        let key = self.key.as_ref().filter(|_| is_digits(trimmed))?;
        self.pattern.render(Some(key), trimmed.parse().ok()?).ok()
    }

    fn identity_key(&self, reference: &str) -> Option<String> {
        if is_digits(reference) {
            let number: u64 = reference.parse().ok()?;
            return Some(
                self.pattern
                    .render(self.key.as_ref(), number)
                    .unwrap_or_else(|_| self.padded(number)),
            );
        }
        IdParts::parse(&self.pattern, reference)
            .and_then(|parts| self.joined(&parts))
    }

    fn filename_identity(&self, filename: &str) -> Option<FilenameIdentity> {
        let held = self.read_held_prefix(filename)?;
        Some(FilenameIdentity {
            identity_key: self.joined(held.parts())?,
            id_end: held.id_end(),
        })
    }

    fn read_held(&self, value: &str) -> Option<IdParts> {
        IdParts::parse(&self.pattern, value).or_else(|| self.bare_number(value))
    }

    fn read_held_prefix(&self, filename: &str) -> Option<HeldPrefix> {
        IdParts::parse_prefix(&self.pattern, filename).or_else(|| {
            let digits = legacy_prefix(filename)?;
            Some(HeldPrefix::new(self.bare_number(digits)?, digits.len()))
        })
    }

    fn holds(&self, held: &str, id: &IdParts) -> bool {
        self.read_held(held)
            .is_some_and(|reading| reading.same_id(id))
    }

    fn canonicalise(&self, input: &str) -> Result<String, CanonicaliseError> {
        if let Some(parts) = IdParts::parse(&self.pattern, input) {
            return Ok(self.render_keeping_key(&parts)?);
        }
        if is_digits(input) {
            let number =
                input.parse().map_err(|_| CanonicaliseError::NoMatch)?;
            return Ok(self.pattern.render(self.key.as_ref(), number)?);
        }
        Err(CanonicaliseError::UnrecognisedIdShape(input.to_owned()))
    }

    fn key(&self) -> Option<&IdKey> {
        self.key.as_ref()
    }

    fn is_bare_id(&self, token: &str) -> bool {
        self.pattern.read_exact(token).is_some()
    }

    fn tracker_keys(&self) -> TrackerKeys {
        self.tracker_keys
    }
}
