use std::collections::HashMap;
use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Serialize;

use crate::indexer::{
    facets_for, LatestPreview, LibraryAggregates, PerTypeAggregate, Selection,
};
use crate::server::AppState;
use corpus::doc_type::LibraryPhase;
use corpus::DocTypeCatalogue;
use corpus::DocTypeKey;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStructureResponse {
    pub phases: Vec<Phase>,
    /// Virtual templates entry, emitted at the top level (templates has no phase).
    pub templates: Option<LibraryDocType>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Phase {
    pub id: String,
    pub label: String,
    pub doc_types: Vec<LibraryDocType>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDocType {
    #[serde(with = "crate::doc_type_serde")]
    pub id: DocTypeKey,
    pub label: String,
    pub count: usize,
    pub filtered_count: usize,
    pub latest: Option<LatestPreviewWire>,
    pub filter_facets: Vec<Facet>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LatestPreviewWire {
    pub title: String,
    pub slug: Option<String>,
    pub modified_at: i64,
}

impl From<&LatestPreview> for LatestPreviewWire {
    fn from(value: &LatestPreview) -> Self {
        Self {
            title: value.title.clone(),
            slug: value.slug.clone(),
            modified_at: value.modified_at,
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Facet {
    pub id: String,
    pub label: String,
    pub options: Vec<FacetOption>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FacetOption {
    pub id: String,
    pub label: String,
    pub count: usize,
}

const PHASES: &[(LibraryPhase, &str, &str)] = &[
    (LibraryPhase::Define, "define", "Define"),
    (LibraryPhase::Discover, "discover", "Discover"),
    (LibraryPhase::Build, "build", "Build"),
    (LibraryPhase::Ship, "ship", "Ship"),
    (LibraryPhase::Operate, "operate", "Operate"),
    (LibraryPhase::Remember, "remember", "Remember"),
];

pub(crate) async fn library_structure(
    State(state): State<Arc<AppState>>,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> Json<LibraryStructureResponse> {
    let selection = parse_selection_query(
        &state.cfg.doc_types,
        raw.as_deref().unwrap_or(""),
    );
    let agg = state.indexer.library_aggregates(&selection).await;
    Json(build_structure(&state.cfg, &agg))
}

/// Parses repeated query keys of the form `selection[<type>][<facet>]=<option>`
/// into a `Selection`. URL-decoded values are returned verbatim. Repeated keys
/// accumulate (OR within a facet). Malformed shapes are silently dropped.
pub(crate) fn parse_selection_query(
    doc_types: &DocTypeCatalogue,
    raw: &str,
) -> Selection {
    let mut out: Selection = HashMap::new();
    for (key, value) in form_urlencoded::parse(raw.as_bytes()) {
        let Some(rest) = key.strip_prefix("selection[") else {
            continue;
        };
        let Some((type_token, rest)) = rest.split_once(']') else {
            continue;
        };
        let Some(rest) = rest.strip_prefix('[') else {
            continue;
        };
        let Some((facet_id, tail)) = rest.split_once(']') else {
            continue;
        };
        if !tail.is_empty() {
            continue;
        }
        if value.is_empty() {
            continue;
        }
        let Some(doc_type) = doc_types.from_wire_token(type_token) else {
            continue;
        };
        out.entry(doc_type)
            .or_default()
            .entry(facet_id.to_string())
            .or_default()
            .push(value.into_owned());
    }
    out
}

fn build_structure(
    cfg: &crate::config::Config,
    agg: &LibraryAggregates,
) -> LibraryStructureResponse {
    let catalogue = &cfg.doc_types;
    let phases = PHASES
        .iter()
        .map(|(phase, id, label)| Phase {
            id: (*id).to_string(),
            label: (*label).to_string(),
            doc_types: placed_in(catalogue, *phase)
                .into_iter()
                .map(|dt| build_doc_type(cfg, agg, dt))
                .collect(),
        })
        .collect();

    LibraryStructureResponse {
        phases,
        templates: catalogue
            .doc_types()
            .iter()
            .find(|kind| kind.is_virtual())
            .map(|templates| build_doc_type(cfg, agg, *templates)),
    }
}

fn placed_in(
    catalogue: &DocTypeCatalogue,
    phase: LibraryPhase,
) -> Vec<DocTypeKey> {
    let mut placed: Vec<(u8, DocTypeKey)> = catalogue
        .doc_types()
        .iter()
        .filter_map(|kind| {
            kind.library_placement()
                .filter(|placement| placement.phase == phase)
                .map(|placement| (placement.position, *kind))
        })
        .collect();
    placed.sort_by_key(|(position, _)| *position);
    placed.into_iter().map(|(_, kind)| kind).collect()
}

fn build_doc_type(
    cfg: &crate::config::Config,
    agg: &LibraryAggregates,
    doc_type: DocTypeKey,
) -> LibraryDocType {
    let per = agg.per_type.get(&doc_type);
    LibraryDocType {
        id: doc_type,
        label: doc_type.label().to_string(),
        count: per.map_or(0, |p| p.count),
        filtered_count: per.map_or(0, |p| p.filtered_count),
        latest: per
            .and_then(|p| p.latest.as_ref())
            .map(LatestPreviewWire::from),
        filter_facets: build_facets(cfg, per, doc_type),
    }
}

fn build_facets(
    cfg: &crate::config::Config,
    per: Option<&PerTypeAggregate>,
    doc_type: DocTypeKey,
) -> Vec<Facet> {
    let Some(per) = per else { return Vec::new() };
    facets_for(doc_type)
        .iter()
        .map(|facet_id| Facet {
            id: (*facet_id).to_string(),
            label: facet_label(facet_id),
            options: per
                .facet_options
                .get(*facet_id)
                .map(|m| {
                    m.iter()
                        .map(|(id, count)| FacetOption {
                            id: id.clone(),
                            label: facet_option_label(
                                cfg, doc_type, facet_id, id,
                            ),
                            count: *count,
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect()
}

fn facet_label(facet_id: &str) -> String {
    match facet_id {
        "status" => "Status".to_string(),
        "clusterSlug" => "Cluster".to_string(),
        "project" => "Project".to_string(),
        other => other.to_string(),
    }
}

fn facet_option_label(
    _cfg: &crate::config::Config,
    _doc_type: DocTypeKey,
    facet_id: &str,
    id: &str,
) -> String {
    match facet_id {
        "status" => humanise_status(id),
        _ => id.to_string(),
    }
}

fn humanise_status(id: &str) -> String {
    let mut chars = id.chars();
    match chars.next() {
        Some(first) => {
            first.to_uppercase().collect::<String>() + chars.as_str()
        }
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection_of(raw: &str) -> Selection {
        parse_selection_query(&DocTypeCatalogue::built_in(), raw)
    }

    #[test]
    fn parses_empty_string_to_empty_selection() {
        assert!(selection_of("").is_empty());
    }

    #[test]
    fn parses_single_selection_key() {
        let sel = selection_of("selection[decisions][status]=open");
        let decisions = sel.get(&DocTypeKey::DECISIONS).unwrap();
        assert_eq!(decisions.get("status").unwrap(), &vec!["open".to_string()]);
    }

    #[test]
    fn parses_repeated_keys_accumulating_into_a_single_facet() {
        let sel = selection_of(
            "selection[decisions][status]=open&selection[decisions][status]=blocked",
        );
        let decisions = sel.get(&DocTypeKey::DECISIONS).unwrap();
        assert_eq!(
            decisions.get("status").unwrap(),
            &vec!["open".to_string(), "blocked".to_string()]
        );
    }

    #[test]
    fn parses_two_facets_under_one_doc_type() {
        let sel = selection_of(
            "selection[decisions][status]=open&selection[decisions][clusterSlug]=foo",
        );
        let decisions = sel.get(&DocTypeKey::DECISIONS).unwrap();
        assert_eq!(decisions.get("status").unwrap(), &vec!["open".to_string()]);
        assert_eq!(
            decisions.get("clusterSlug").unwrap(),
            &vec!["foo".to_string()]
        );
    }

    #[test]
    fn empty_value_is_silently_dropped() {
        let sel = selection_of("selection[decisions][status]=");
        assert!(sel.is_empty());
    }

    #[test]
    fn url_encoded_value_round_trips() {
        let sel = selection_of("selection[decisions][clusterSlug]=foo%20bar");
        let decisions = sel.get(&DocTypeKey::DECISIONS).unwrap();
        assert_eq!(
            decisions.get("clusterSlug").unwrap(),
            &vec!["foo bar".to_string()]
        );
    }

    #[test]
    fn percent_encoded_reserved_characters_round_trip() {
        let sel = selection_of("selection[decisions][clusterSlug]=a%2Cb");
        let decisions = sel.get(&DocTypeKey::DECISIONS).unwrap();
        assert_eq!(
            decisions.get("clusterSlug").unwrap(),
            &vec!["a,b".to_string()]
        );
    }

    #[test]
    fn unknown_doc_type_is_silently_dropped() {
        let sel = selection_of("selection[bogus][status]=open");
        assert!(sel.is_empty());
    }

    #[test]
    fn malformed_shapes_are_silently_dropped() {
        assert!(selection_of("selection[decisions]=open").is_empty());
        assert!(selection_of("selectionopen").is_empty());
        assert!(selection_of("[decisions][status]=open").is_empty());
    }

    #[test]
    fn unrelated_query_params_are_ignored() {
        let sel = selection_of("other=value&selection[decisions][status]=open");
        assert_eq!(sel.len(), 1);
        let decisions = sel.get(&DocTypeKey::DECISIONS).unwrap();
        assert_eq!(decisions.get("status").unwrap(), &vec!["open".to_string()]);
    }

    #[test]
    fn duplicate_options_preserve_duplicates() {
        let sel = selection_of(
            "selection[decisions][status]=open&selection[decisions][status]=open",
        );
        let decisions = sel.get(&DocTypeKey::DECISIONS).unwrap();
        assert_eq!(
            decisions.get("status").unwrap(),
            &vec!["open".to_string(), "open".to_string()]
        );
    }
}
