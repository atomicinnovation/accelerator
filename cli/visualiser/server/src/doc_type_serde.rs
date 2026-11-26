//! Serialises a `corpus::DocTypeKey` as its wire token. There is no
//! `deserialize`: handlers resolve wire input against the catalogue the
//! server was composed with.

use serde::Serializer;

pub fn serialize<S: Serializer>(
    kind: &corpus::DocTypeKey,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(kind.wire_token())
}
