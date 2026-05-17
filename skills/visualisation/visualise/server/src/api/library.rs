use std::collections::HashMap;
use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Serialize;

use crate::docs::DocTypeKey;
use crate::indexer::{facets_for, LatestPreview, LibraryAggregates, PerTypeAggregate, Selection};
use crate::server::AppState;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStructureResponse {
    pub phases: Vec<Phase>,
    /// Virtual templates entry, emitted at the top level (templates has no phase).
    pub templates: LibraryDocType,
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

/// Static phase membership. Centralises what was previously the client-side
/// `PHASE_DOC_TYPES` table. If phase configuration needs to be data-driven
/// later, this is the seam to extend.
const PHASES: &[(&str, &str, &[DocTypeKey])] = &[
    (
        "define",
        "Define",
        &[DocTypeKey::WorkItems, DocTypeKey::WorkItemReviews],
    ),
    (
        "discover",
        "Discover",
        &[
            DocTypeKey::DesignInventories,
            DocTypeKey::DesignGaps,
            DocTypeKey::Research,
        ],
    ),
    (
        "build",
        "Build",
        &[
            DocTypeKey::Plans,
            DocTypeKey::PlanReviews,
            DocTypeKey::Validations,
        ],
    ),
    (
        "ship",
        "Ship",
        &[DocTypeKey::Prs, DocTypeKey::PrReviews],
    ),
    (
        "remember",
        "Remember",
        &[DocTypeKey::Decisions, DocTypeKey::Notes],
    ),
];

pub(crate) async fn library_structure(
    State(state): State<Arc<AppState>>,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> Json<LibraryStructureResponse> {
    let selection = parse_selection_query(raw.as_deref().unwrap_or(""));
    let agg = state
        .indexer
        .library_aggregates(&state.cfg, &selection)
        .await;
    Json(build_structure(&state.cfg, &agg))
}

/// Parses repeated query keys of the form `selection[<type>][<facet>]=<option>`
/// into a `Selection`. URL-decoded values are returned verbatim. Repeated keys
/// accumulate (OR within a facet). Malformed shapes are silently dropped.
pub(crate) fn parse_selection_query(raw: &str) -> Selection {
    let mut out: Selection = HashMap::new();
    for (key, value) in form_urlencoded::parse(raw.as_bytes()) {
        let Some(rest) = key.strip_prefix("selection[") else { continue };
        let Some((type_token, rest)) = rest.split_once(']') else { continue };
        let Some(rest) = rest.strip_prefix('[') else { continue };
        let Some((facet_id, tail)) = rest.split_once(']') else { continue };
        if !tail.is_empty() {
            continue;
        }
        if value.is_empty() {
            continue;
        }
        let Some(doc_type) = DocTypeKey::from_wire_str(type_token) else { continue };
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
    let phases = PHASES
        .iter()
        .map(|(id, label, doc_types)| Phase {
            id: (*id).to_string(),
            label: (*label).to_string(),
            doc_types: doc_types
                .iter()
                .map(|dt| build_doc_type(cfg, agg, *dt))
                .collect(),
        })
        .collect();

    LibraryStructureResponse {
        phases,
        templates: build_doc_type(cfg, agg, DocTypeKey::Templates),
    }
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
        count: per.map(|p| p.count).unwrap_or(0),
        filtered_count: per.map(|p| p.filtered_count).unwrap_or(0),
        latest: per.and_then(|p| p.latest.as_ref()).map(LatestPreviewWire::from),
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
                            label: facet_option_label(cfg, doc_type, facet_id, id),
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
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_string_to_empty_selection() {
        assert!(parse_selection_query("").is_empty());
    }

    #[test]
    fn parses_single_selection_key() {
        let sel = parse_selection_query("selection[decisions][status]=open");
        let decisions = sel.get(&DocTypeKey::Decisions).unwrap();
        assert_eq!(decisions.get("status").unwrap(), &vec!["open".to_string()]);
    }

    #[test]
    fn parses_repeated_keys_accumulating_into_a_single_facet() {
        let sel = parse_selection_query(
            "selection[decisions][status]=open&selection[decisions][status]=blocked",
        );
        let decisions = sel.get(&DocTypeKey::Decisions).unwrap();
        assert_eq!(
            decisions.get("status").unwrap(),
            &vec!["open".to_string(), "blocked".to_string()]
        );
    }

    #[test]
    fn parses_two_facets_under_one_doc_type() {
        let sel = parse_selection_query(
            "selection[decisions][status]=open&selection[decisions][clusterSlug]=foo",
        );
        let decisions = sel.get(&DocTypeKey::Decisions).unwrap();
        assert_eq!(decisions.get("status").unwrap(), &vec!["open".to_string()]);
        assert_eq!(
            decisions.get("clusterSlug").unwrap(),
            &vec!["foo".to_string()]
        );
    }

    #[test]
    fn empty_value_is_silently_dropped() {
        let sel = parse_selection_query("selection[decisions][status]=");
        assert!(sel.is_empty());
    }

    #[test]
    fn url_encoded_value_round_trips() {
        let sel = parse_selection_query("selection[decisions][clusterSlug]=foo%20bar");
        let decisions = sel.get(&DocTypeKey::Decisions).unwrap();
        assert_eq!(
            decisions.get("clusterSlug").unwrap(),
            &vec!["foo bar".to_string()]
        );
    }

    #[test]
    fn percent_encoded_reserved_characters_round_trip() {
        let sel = parse_selection_query("selection[decisions][clusterSlug]=a%2Cb");
        let decisions = sel.get(&DocTypeKey::Decisions).unwrap();
        assert_eq!(
            decisions.get("clusterSlug").unwrap(),
            &vec!["a,b".to_string()]
        );
    }

    #[test]
    fn unknown_doc_type_is_silently_dropped() {
        let sel = parse_selection_query("selection[bogus][status]=open");
        assert!(sel.is_empty());
    }

    #[test]
    fn malformed_shapes_are_silently_dropped() {
        assert!(parse_selection_query("selection[decisions]=open").is_empty());
        assert!(parse_selection_query("selectionopen").is_empty());
        assert!(parse_selection_query("[decisions][status]=open").is_empty());
    }

    #[test]
    fn unrelated_query_params_are_ignored() {
        let sel = parse_selection_query("other=value&selection[decisions][status]=open");
        assert_eq!(sel.len(), 1);
        let decisions = sel.get(&DocTypeKey::Decisions).unwrap();
        assert_eq!(decisions.get("status").unwrap(), &vec!["open".to_string()]);
    }

    #[test]
    fn duplicate_options_preserve_duplicates() {
        let sel = parse_selection_query(
            "selection[decisions][status]=open&selection[decisions][status]=open",
        );
        let decisions = sel.get(&DocTypeKey::Decisions).unwrap();
        assert_eq!(
            decisions.get("status").unwrap(),
            &vec!["open".to_string(), "open".to_string()]
        );
    }
}
