#![allow(clippy::expect_used)]

use research::request::ApiKey;
use research::request::ArxivId;
use research::request::ArxivQuery;
use research::request::ArxivRequest;
use research::request::Doi;
use research::request::Endpoint;
use research::request::Family;
use research::request::FetchRequest;
use research::request::KeySource;
use research::request::Limit;
use research::request::OpenAlexId;
use research::request::OpenAlexQuery;
use research::request::OpenAlexRequest;
use research::request::RequestError;
use research::request::UpstreamRequest;
use research::request::Verb;

fn terms(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

#[test]
fn a_family_parses_only_from_its_exact_name() {
    assert_eq!(Family::parse("openalex"), Ok(Family::OpenAlex));
    assert_eq!(Family::parse("arxiv"), Ok(Family::Arxiv));
    assert_eq!(
        Family::parse("openAlex").map_err(|error| error.to_string()),
        Err(
            "E_RESEARCH_USAGE: unknown family 'openAlex' (expected openalex \
             or arxiv)"
                .to_owned()
        )
    );
}

#[test]
fn a_family_names_itself_for_callers_and_for_readers() {
    assert_eq!(Family::OpenAlex.code(), "openalex");
    assert_eq!(Family::Arxiv.code(), "arxiv");
    assert_eq!(Family::OpenAlex.to_string(), "OpenAlex");
    assert_eq!(Family::Arxiv.to_string(), "arXiv");
}

#[test]
fn a_verb_names_itself_as_callers_spell_it() {
    assert_eq!(Verb::Search.code(), "search");
    assert_eq!(Verb::Lookup.code(), "lookup");
}

#[test]
fn a_limit_defaults_to_ten() {
    assert_eq!(Limit::default().get(), 10);
}

#[test]
fn a_limit_accepts_one_to_twenty_five() {
    assert_eq!(Limit::parse("1").map(Limit::get), Ok(1));
    assert_eq!(Limit::parse("25").map(Limit::get), Ok(25));
}

#[test]
fn a_limit_outside_its_range_names_the_rejected_value() {
    for rejected in ["0", "26", "50", "-1", "ten", ""] {
        assert_eq!(
            Limit::parse(rejected).map_err(|error| error.to_string()),
            Err(format!(
                "E_RESEARCH_USAGE: --limit must be 1–25 (got {rejected})"
            )),
        );
    }
}

#[test]
fn a_new_style_arxiv_id_parses_with_or_without_its_version() {
    let versioned = ArxivId::parse("2608.21129v2").expect("valid");
    assert_eq!(versioned.unversioned(), "2608.21129");
    assert_eq!(versioned.version(), Some(2));

    let bare = ArxivId::parse("2101.0001").expect("valid");
    assert_eq!(bare.unversioned(), "2101.0001");
    assert_eq!(bare.version(), None);
}

#[test]
fn an_old_style_arxiv_id_parses_with_its_archive_and_subject_class() {
    let plain = ArxivId::parse("hep-th/9901001v3").expect("valid");
    assert_eq!(plain.unversioned(), "hep-th/9901001");
    assert_eq!(plain.version(), Some(3));

    let classed = ArxivId::parse("math.GT/0309136").expect("valid");
    assert_eq!(classed.unversioned(), "math.GT/0309136");
}

#[test]
fn a_malformed_arxiv_id_is_refused_naming_it() {
    for malformed in [
        "2608.2112x",
        "2608.211",
        "2608.211290",
        "26081.21129",
        "2608.21129v",
        "2608.21129v2x",
        "hep-th/990100",
        "HEP-TH/9901001",
        "math.gt/0309136",
        "/9901001",
        "",
    ] {
        assert_eq!(
            ArxivId::parse(malformed).map_err(|error| error.to_string()),
            Err(format!(
                "E_ARXIV_ID_MALFORMED: expected 2608.21129[vN] or \
                 archive/1234567 (got '{malformed}')"
            )),
        );
    }
}

#[test]
fn an_arxiv_id_is_read_from_its_abstract_page_url() {
    let id = ArxivId::from_abstract_url("http://arxiv.org/abs/2608.21129v2")
        .expect("valid");
    assert_eq!(id.unversioned(), "2608.21129");
    assert_eq!(id.version(), Some(2));
    assert!(ArxivId::from_abstract_url(
        "https://arxiv.org/abs/hep-th/9901001v1"
    )
    .is_some());
    assert!(
        ArxivId::from_abstract_url("https://evil.example/abs/2608.21129")
            .is_none()
    );
    assert!(
        ArxivId::from_abstract_url("http://arxiv.org/abs/nonsense").is_none()
    );
}

#[test]
fn a_doi_is_anchored_on_its_registrant_prefix() {
    assert!(Doi::parse("10.7717/peerj.4375").is_some());
    assert!(Doi::parse("10.123/too-short-registrant").is_none());
    assert!(Doi::parse("10.1234567890/too-long-registrant").is_none());
    assert!(Doi::parse("11.1234/wrong-directory").is_none());
    assert!(Doi::parse("10.1234/").is_none());
    assert!(Doi::parse("10.1234/has space").is_none());
    assert!(Doi::parse("x10.1234/prefixed").is_none());
}

#[test]
fn a_doi_with_a_dot_segment_is_refused() {
    assert!(Doi::parse("10.1234/../works").is_none());
    assert!(Doi::parse("10.1234/a/./b").is_none());
    assert!(Doi::parse("10.1234/a/..").is_none());
    assert!(Doi::parse("10.1234/a..b").is_some());
}

#[test]
fn a_doi_percent_encodes_outside_the_unreserved_set_and_slash() {
    let doi = Doi::parse("10.1002/(SICI)1097<x>;2-#").expect("valid");
    assert_eq!(doi.encoded(), "10.1002/%28SICI%291097%3Cx%3E%3B2-%23");
    assert_eq!(
        doi.url(),
        "https://doi.org/10.1002/%28SICI%291097%3Cx%3E%3B2-%23"
    );
}

#[test]
fn a_doi_encodes_multibyte_text_byte_by_byte() {
    let doi = Doi::parse("10.1234/café").expect("valid");
    assert_eq!(doi.encoded(), "10.1234/caf%C3%A9");
}

#[test]
fn an_openalex_id_accepts_a_work_id_in_bare_or_url_form() {
    for form in ["W2741809807", "https://openalex.org/W2741809807"] {
        assert!(matches!(
            OpenAlexId::parse(form),
            Ok(OpenAlexId::Work(id)) if id.as_str() == "W2741809807"
        ));
    }
}

#[test]
fn an_openalex_id_accepts_a_doi_in_bare_prefixed_or_url_form() {
    let expected = Doi::parse("10.7717/peerj.4375").expect("valid");
    for form in [
        "10.7717/peerj.4375",
        "doi:10.7717/peerj.4375",
        "https://doi.org/10.7717/peerj.4375",
    ] {
        assert_eq!(
            OpenAlexId::parse(form),
            Ok(OpenAlexId::Doi(expected.clone()))
        );
    }
}

#[test]
fn a_malformed_openalex_id_is_refused_naming_it() {
    for malformed in [
        "W",
        "w123",
        "W12x",
        "https://openalex.org/A123",
        "10.1234/../x",
    ] {
        assert_eq!(
            OpenAlexId::parse(malformed).map_err(|error| error.to_string()),
            Err(format!(
                "E_OPENALEX_ID_MALFORMED: expected W123, \
                 https://openalex.org/W123, or a DOI (got '{malformed}')"
            )),
        );
    }
}

#[test]
fn an_openalex_query_replaces_filter_syntax_with_spaces() {
    let query =
        OpenAlexQuery::parse("graph, neural|networks: a!b").expect("non-empty");
    assert_eq!(query.as_str(), "graph neural networks a b");
}

#[test]
fn an_openalex_query_empty_after_normalisation_is_refused() {
    assert_eq!(
        OpenAlexQuery::parse(" ,|!: ").map_err(|error| error.to_string()),
        Err(
            "E_RESEARCH_USAGE: the query is empty once reserved characters \
             are removed"
                .to_owned()
        )
    );
}

#[test]
fn an_arxiv_query_joins_all_field_clauses_with_and() {
    let query = ArxivQuery::parse("graph neural networks").expect("non-empty");
    assert_eq!(query.as_str(), "all:graph AND all:neural AND all:networks");
}

#[test]
fn an_arxiv_query_drops_grouping_quotes_and_operator_words() {
    let query =
        ArxivQuery::parse("(\"graph\" OR 'neural') AND nets ANDNOT trees")
            .expect("non-empty");
    assert_eq!(
        query.as_str(),
        "all:graph AND all:neural AND all:nets AND all:trees"
    );
}

#[test]
fn an_arxiv_query_empty_after_normalisation_is_refused() {
    assert_eq!(
        ArxivQuery::parse("( \"OR\" AND )").map_err(|error| error.to_string()),
        Err(
            "E_RESEARCH_USAGE: the query is empty once reserved characters \
             are removed"
                .to_owned()
        )
    );
}

#[test]
fn a_search_joins_its_terms_with_single_spaces() {
    let request = FetchRequest::parse(
        "openalex",
        "search",
        &terms(&["graph", "neural"]),
        None,
    )
    .expect("valid");
    assert_eq!(
        request,
        FetchRequest::OpenAlex(OpenAlexRequest::Search {
            query: OpenAlexQuery::parse("graph neural").expect("valid"),
            limit: Limit::default(),
        })
    );
}

#[test]
fn a_search_honours_its_limit() {
    let request =
        FetchRequest::parse("arxiv", "search", &terms(&["graphs"]), Some("3"))
            .expect("valid");
    assert_eq!(
        request,
        FetchRequest::Arxiv(ArxivRequest::Search {
            query: ArxivQuery::parse("graphs").expect("valid"),
            limit: Limit::parse("3").expect("valid"),
        })
    );
}

#[test]
fn a_lookup_parses_its_family_identifier() {
    assert_eq!(
        FetchRequest::parse("arxiv", "lookup", &terms(&["2608.21129v2"]), None),
        Ok(FetchRequest::Arxiv(ArxivRequest::Lookup(
            ArxivId::parse("2608.21129v2").expect("valid")
        )))
    );
    assert_eq!(
        FetchRequest::parse("openalex", "lookup", &terms(&["W1"]), None),
        Ok(FetchRequest::OpenAlex(OpenAlexRequest::Lookup(
            OpenAlexId::parse("W1").expect("valid")
        )))
    );
}

#[test]
fn a_request_refuses_each_usage_error_naming_the_bad_value() {
    type UsageCase<'a> =
        (&'a str, &'a str, &'a [&'a str], Option<&'a str>, &'a str);
    let cases: [UsageCase<'_>; 5] = [
        (
            "openalex",
            "fetch",
            &["x"],
            None,
            "E_RESEARCH_USAGE: unknown verb 'fetch' (expected search or \
             lookup)",
        ),
        (
            "openalex",
            "search",
            &[],
            None,
            "E_RESEARCH_USAGE: search needs a query",
        ),
        (
            "arxiv",
            "lookup",
            &[],
            None,
            "E_RESEARCH_USAGE: lookup needs an ID",
        ),
        (
            "openalex",
            "search",
            &["x"],
            Some("26"),
            "E_RESEARCH_USAGE: --limit must be 1–25 (got 26)",
        ),
        (
            "crossref",
            "search",
            &["x"],
            None,
            "E_RESEARCH_USAGE: unknown family 'crossref' (expected openalex \
             or arxiv)",
        ),
    ];
    for (family, verb, words, limit, message) in cases {
        assert_eq!(
            FetchRequest::parse(family, verb, &terms(words), limit)
                .map_err(|error| error.to_string()),
            Err(message.to_owned()),
        );
    }
}

#[test]
fn a_usage_error_is_distinguishable_from_a_malformed_identifier() {
    assert!(matches!(
        FetchRequest::parse("arxiv", "lookup", &terms(&["nope"]), None),
        Err(RequestError::MalformedArxivId(_))
    ));
}

#[test]
fn an_openalex_search_encodes_the_query_into_one_filter_value() {
    let request = OpenAlexRequest::Search {
        query: OpenAlexQuery::parse("graphs &per_page=200 #x+y")
            .expect("valid"),
        limit: Limit::parse("5").expect("valid"),
    };
    let upstream =
        request.upstream(&Endpoint::new("https://api.openalex.org/"), None);
    assert_eq!(
        upstream.url(),
        "https://api.openalex.org/works?filter=title_and_abstract.search:\
         graphs+%26per_page%3D200+%23x%2By&per_page=5&select=id,doi,\
         display_name,type,authorships,primary_location,is_retracted,\
         abstract_inverted_index"
    );
}

#[test]
fn an_openalex_lookup_addresses_the_work_or_its_encoded_doi() {
    let api = Endpoint::new("https://api.openalex.org");
    let by_work =
        OpenAlexRequest::Lookup(OpenAlexId::parse("W7").expect("valid"));
    assert!(by_work
        .upstream(&api, None)
        .url()
        .starts_with("https://api.openalex.org/works/W7?select=id,doi,"));

    let by_doi = OpenAlexRequest::Lookup(
        OpenAlexId::parse("10.1002/(SICI)1").expect("valid"),
    );
    assert!(by_doi.upstream(&api, None).url().starts_with(
        "https://api.openalex.org/works/doi:10.1002/%28SICI%291?select="
    ));
}

#[test]
fn an_openalex_request_carries_a_bearer_key_only_when_one_resolved() {
    let request =
        OpenAlexRequest::Lookup(OpenAlexId::parse("W7").expect("valid"));
    let api = Endpoint::new("https://api.openalex.org");
    let key = ApiKey::new(
        "secret-value".to_owned(),
        KeySource::new("ACCELERATOR_OPENALEX_API_KEY"),
    );

    assert_eq!(request.upstream(&api, None).bearer(), None);
    let keyed = request.upstream(&api, Some(&key));
    assert_eq!(keyed.bearer(), Some("secret-value"));
    assert!(!keyed.url().contains("secret-value"));
}

#[test]
fn a_key_never_renders_its_secret() {
    let key = ApiKey::new(
        "secret-value".to_owned(),
        KeySource::new("openalex.api_key"),
    );
    let request =
        OpenAlexRequest::Lookup(OpenAlexId::parse("W7").expect("valid"))
            .upstream(&Endpoint::new("https://api.openalex.org"), Some(&key));
    assert!(!format!("{key:?}").contains("secret-value"));
    assert!(!format!("{request:?}").contains("secret-value"));
    assert_eq!(key.source().to_string(), "openalex.api_key");
}

#[test]
fn every_request_identifies_the_client() {
    let openalex =
        OpenAlexRequest::Lookup(OpenAlexId::parse("W7").expect("valid"))
            .upstream(&Endpoint::new("https://api.openalex.org"), None);
    let user_agent =
        format!("accelerator-research/{}", env!("CARGO_PKG_VERSION"));
    assert_eq!(openalex.headers(), [("User-Agent", user_agent.clone())]);

    let arxiv = ArxivRequest::Lookup(ArxivId::parse("2608.21129").expect("ok"))
        .upstream(&Endpoint::new("https://export.arxiv.org"));
    assert_eq!(
        arxiv.headers(),
        [
            ("User-Agent", user_agent),
            ("Accept", "application/atom+xml".to_owned()),
        ]
    );
    assert_eq!(arxiv.bearer(), None);
}

#[test]
fn an_arxiv_search_translates_the_query_and_limit() {
    let request = ArxivRequest::Search {
        query: ArxivQuery::parse("graph neural").expect("valid"),
        limit: Limit::parse("3").expect("valid"),
    };
    assert_eq!(
        request
            .upstream(&Endpoint::new("https://export.arxiv.org"))
            .url(),
        "https://export.arxiv.org/api/query?search_query=all:graph+AND+\
         all:neural&max_results=3&sortBy=relevance"
    );
}

#[test]
fn an_arxiv_lookup_asks_for_the_unversioned_id() {
    let request =
        ArxivRequest::Lookup(ArxivId::parse("2608.21129v2").expect("valid"));
    assert_eq!(
        request
            .upstream(&Endpoint::new("https://export.arxiv.org"))
            .url(),
        "https://export.arxiv.org/api/query?id_list=2608.21129"
    );
}

#[test]
fn a_withdrawal_confirmation_asks_oai_for_the_raw_record() {
    let id = ArxivId::parse("hep-th/9901001v2").expect("valid");
    assert_eq!(
        UpstreamRequest::withdrawal_confirmation(
            &Endpoint::new("https://oaipmh.arxiv.org"),
            &id
        )
        .url(),
        "https://oaipmh.arxiv.org/oai?verb=GetRecord&identifier=\
         oai:arXiv.org:hep-th/9901001&metadataPrefix=arXivRaw"
    );
}

#[test]
fn the_production_endpoints_are_the_public_apis() {
    assert_eq!(Endpoint::openalex().as_str(), "https://api.openalex.org");
    assert_eq!(Endpoint::arxiv_api().as_str(), "https://export.arxiv.org");
    assert_eq!(Endpoint::arxiv_oai().as_str(), "https://oaipmh.arxiv.org");
}
