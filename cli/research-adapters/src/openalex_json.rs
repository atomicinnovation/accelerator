//! OpenAlex's JSON works, decoded into the domain's typed inputs.
//!
//! Only the fields the `select` projection asks for are read. Nullable
//! fields decode as absent, but a work without an `id` is undecodable: it
//! could never be cited.

use std::collections::BTreeMap;

use research::fetch::DecodeFailure;
use research::fetch::OpenAlexDecoder;
use research::openalex::Location;
use research::openalex::Source;
use research::openalex::Work;
use serde::de::DeserializeOwned;
use serde::Deserialize;

pub struct JsonOpenAlexDecoder;

impl OpenAlexDecoder for JsonOpenAlexDecoder {
    fn works(&self, body: &[u8]) -> Result<Vec<Work>, DecodeFailure> {
        let page: Page = decode(body)?;
        Ok(page.results.into_iter().map(Work::from).collect())
    }

    fn work(&self, body: &[u8]) -> Result<Work, DecodeFailure> {
        decode::<WorkJson>(body).map(Work::from)
    }
}

fn decode<T: DeserializeOwned>(body: &[u8]) -> Result<T, DecodeFailure> {
    serde_json::from_slice(body).map_err(|_| DecodeFailure::Undecodable)
}

#[derive(Deserialize)]
struct Page {
    results: Vec<WorkJson>,
}

#[derive(Deserialize)]
struct WorkJson {
    id: String,
    doi: Option<String>,
    display_name: Option<String>,
    #[serde(rename = "type")]
    work_type: Option<String>,
    authorships: Option<Vec<Authorship>>,
    primary_location: Option<LocationJson>,
    is_retracted: Option<bool>,
    abstract_inverted_index: Option<BTreeMap<String, Vec<usize>>>,
}

#[derive(Deserialize)]
struct Authorship {
    author: Option<Author>,
}

#[derive(Deserialize)]
struct Author {
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct LocationJson {
    version: Option<String>,
    source: Option<SourceJson>,
}

#[derive(Deserialize)]
struct SourceJson {
    display_name: Option<String>,
    #[serde(rename = "type")]
    source_type: Option<String>,
    is_core: Option<bool>,
    listed_in: Option<Vec<String>>,
}

impl From<WorkJson> for Work {
    fn from(work: WorkJson) -> Self {
        Self {
            id: work.id,
            doi: work.doi,
            title: work.display_name,
            work_type: work.work_type.unwrap_or_default(),
            authors: work
                .authorships
                .unwrap_or_default()
                .into_iter()
                .filter_map(|authorship| authorship.author?.display_name)
                .collect(),
            primary_location: work.primary_location.map(Location::from),
            is_retracted: work.is_retracted.unwrap_or_default(),
            abstract_inverted_index: work
                .abstract_inverted_index
                .map(|index| index.into_iter().collect()),
        }
    }
}

impl From<LocationJson> for Location {
    fn from(location: LocationJson) -> Self {
        Self {
            version: location.version,
            source: location.source.map(Source::from),
        }
    }
}

impl From<SourceJson> for Source {
    fn from(source: SourceJson) -> Self {
        Self {
            display_name: source.display_name,
            source_type: source.source_type,
            is_core: source.is_core.unwrap_or_default(),
            listed_in: source.listed_in.unwrap_or_default(),
        }
    }
}
