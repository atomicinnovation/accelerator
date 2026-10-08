//! A key the tracker answers an issue under, once checked to be shaped like
//! one, so it can name the files and directories a work item takes.

use std::fmt;

use tracker::ExternalId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackerKey(String);

impl TrackerKey {
    #[must_use]
    pub fn parse(token: &str) -> Option<Self> {
        corpus::is_tracker_key(token).then(|| Self(token.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TrackerKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&TrackerKey> for ExternalId {
    fn from(key: &TrackerKey) -> Self {
        Self::new(key.0.clone())
    }
}

#[cfg(test)]
mod tests {
    use tracker::ExternalId;

    use super::TrackerKey;

    #[test]
    fn only_a_key_shaped_token_is_a_tracker_key() {
        assert_eq!(
            TrackerKey::parse("MY_PROJ-12").map(|key| key.as_str().to_owned()),
            Some("MY_PROJ-12".to_owned())
        );
        for token in ["PP-1/../..", "../../etc", "draft-k7mq3x", "0042", ""] {
            assert!(TrackerKey::parse(token).is_none(), "{token}");
        }
    }

    #[test]
    fn a_tracker_key_is_the_issue_id_the_tracker_reads() {
        let key = TrackerKey::parse("PP-760")
            .unwrap_or_else(|| unreachable!("a tracker key"));

        assert_eq!(
            ExternalId::from(&key),
            ExternalId::new("PP-760".to_owned())
        );
        assert_eq!(key.to_string(), "PP-760");
    }
}
