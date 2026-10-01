//! What a caller asks a source for, validated before anything is sent, and
//! the upstream request each ask becomes.

use std::fmt;

const USER_AGENT: &str =
    concat!("accelerator-research/", env!("CARGO_PKG_VERSION"));
const ATOM: &str = "application/atom+xml";
const OPENALEX_SELECT: &str = "id,doi,display_name,type,authorships,\
                               primary_location,is_retracted,\
                               abstract_inverted_index";

/// Why a request was refused before any upstream call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestError {
    UnknownFamily(String),
    UnknownVerb(String),
    LimitOutOfRange(String),
    MissingQuery,
    MissingId,
    EmptyQuery,
    MalformedArxivId(String),
    MalformedOpenAlexId(String),
}

impl fmt::Display for RequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFamily(family) => write!(
                formatter,
                "E_RESEARCH_USAGE: unknown family '{family}' (expected \
                 openalex or arxiv)"
            ),
            Self::UnknownVerb(verb) => write!(
                formatter,
                "E_RESEARCH_USAGE: unknown verb '{verb}' (expected search or \
                 lookup)"
            ),
            Self::LimitOutOfRange(limit) => write!(
                formatter,
                "E_RESEARCH_USAGE: --limit must be 1–25 (got {limit})"
            ),
            Self::MissingQuery => {
                formatter.write_str("E_RESEARCH_USAGE: search needs a query")
            }
            Self::MissingId => {
                formatter.write_str("E_RESEARCH_USAGE: lookup needs an ID")
            }
            Self::EmptyQuery => formatter.write_str(
                "E_RESEARCH_USAGE: the query is empty once reserved \
                 characters are removed",
            ),
            Self::MalformedArxivId(id) => write!(
                formatter,
                "E_ARXIV_ID_MALFORMED: expected 2608.21129[vN] or \
                 archive/1234567 (got '{id}')"
            ),
            Self::MalformedOpenAlexId(id) => write!(
                formatter,
                "E_OPENALEX_ID_MALFORMED: expected W123, \
                 https://openalex.org/W123, or a DOI (got '{id}')"
            ),
        }
    }
}

impl std::error::Error for RequestError {}

/// A scholarly source the fetcher can reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    OpenAlex,
    Arxiv,
}

impl Family {
    /// # Errors
    ///
    /// [`RequestError::UnknownFamily`] for anything but an exact family code.
    pub fn parse(raw: &str) -> Result<Self, RequestError> {
        match raw {
            "openalex" => Ok(Self::OpenAlex),
            "arxiv" => Ok(Self::Arxiv),
            _ => Err(RequestError::UnknownFamily(raw.to_owned())),
        }
    }

    /// The name callers pass and machine-readable output carries.
    pub const fn code(self) -> &'static str {
        match self {
            Self::OpenAlex => "openalex",
            Self::Arxiv => "arxiv",
        }
    }
}

impl fmt::Display for Family {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::OpenAlex => "OpenAlex",
            Self::Arxiv => "arXiv",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Search,
    Lookup,
}

impl Verb {
    /// # Errors
    ///
    /// [`RequestError::UnknownVerb`] for anything but `search` or `lookup`.
    pub fn parse(raw: &str) -> Result<Self, RequestError> {
        match raw {
            "search" => Ok(Self::Search),
            "lookup" => Ok(Self::Lookup),
            _ => Err(RequestError::UnknownVerb(raw.to_owned())),
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Lookup => "lookup",
        }
    }
}

/// How many records a search may return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limit(u8);

impl Limit {
    const RANGE: std::ops::RangeInclusive<u8> = 1..=25;

    /// # Errors
    ///
    /// [`RequestError::LimitOutOfRange`] for anything but an integer 1–25.
    pub fn parse(raw: &str) -> Result<Self, RequestError> {
        raw.parse::<u8>()
            .ok()
            .filter(|limit| Self::RANGE.contains(limit))
            .map(Self)
            .ok_or_else(|| RequestError::LimitOutOfRange(raw.to_owned()))
    }

    pub const fn get(self) -> u8 {
        self.0
    }

    pub fn records(self) -> usize {
        usize::from(self.0)
    }
}

impl Default for Limit {
    fn default() -> Self {
        Self(10)
    }
}

/// An arXiv identifier in either the new (`2608.21129v2`) or old
/// (`hep-th/9901001v2`) scheme.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArxivId {
    unversioned: String,
    version: Option<u32>,
}

impl ArxivId {
    /// # Errors
    ///
    /// [`RequestError::MalformedArxivId`] naming the rejected text.
    pub fn parse(raw: &str) -> Result<Self, RequestError> {
        Self::recognise(raw)
            .ok_or_else(|| RequestError::MalformedArxivId(raw.to_owned()))
    }

    /// Reads the identifier out of a feed entry's `http(s)://arxiv.org/abs/…`
    /// URL.
    pub fn from_abstract_url(url: &str) -> Option<Self> {
        ["https://arxiv.org/abs/", "http://arxiv.org/abs/"]
            .iter()
            .find_map(|prefix| url.strip_prefix(prefix))
            .and_then(Self::recognise)
    }

    pub fn unversioned(&self) -> &str {
        &self.unversioned
    }

    pub const fn version(&self) -> Option<u32> {
        self.version
    }

    pub fn abstract_url(&self) -> String {
        format!("https://arxiv.org/abs/{}", self.unversioned)
    }

    fn recognise(raw: &str) -> Option<Self> {
        let (unversioned, version) = split_version(raw)?;
        (is_new_style(unversioned) || is_old_style(unversioned)).then(|| Self {
            unversioned: unversioned.to_owned(),
            version,
        })
    }
}

fn split_version(raw: &str) -> Option<(&str, Option<u32>)> {
    match raw.rsplit_once('v') {
        Some((unversioned, digits))
            if !unversioned.is_empty()
                && unversioned.ends_with(|c: char| c.is_ascii_digit()) =>
        {
            if !all_digits(digits, 1..=usize::MAX) {
                return None;
            }
            Some((unversioned, Some(digits.parse().ok()?)))
        }
        _ => Some((raw, None)),
    }
}

fn is_new_style(candidate: &str) -> bool {
    candidate.split_once('.').is_some_and(|(yymm, number)| {
        all_digits(yymm, 4..=4) && all_digits(number, 4..=5)
    })
}

fn is_old_style(candidate: &str) -> bool {
    let Some((archive, number)) = candidate.split_once('/') else {
        return false;
    };
    let (name, subject_class) = match archive.split_once('.') {
        Some((name, class)) => (name, Some(class)),
        None => (archive, None),
    };
    let name_is_valid = !name.is_empty()
        && name.split('-').all(|part| {
            !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase())
        });
    let class_is_valid = subject_class.is_none_or(|class| {
        class.len() == 2 && class.chars().all(|c| c.is_ascii_uppercase())
    });
    name_is_valid && class_is_valid && all_digits(number, 7..=7)
}

fn all_digits(text: &str, lengths: std::ops::RangeInclusive<usize>) -> bool {
    lengths.contains(&text.len()) && text.chars().all(|c| c.is_ascii_digit())
}

/// A DOI safe to place in a request path or a citation URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doi(String);

impl Doi {
    /// Accepts `10.<4–9 digits>/<suffix>` with no whitespace and no `.` or
    /// `..` path segment.
    pub fn parse(raw: &str) -> Option<Self> {
        let (directory, suffix) = raw.split_once('/')?;
        let registrant = directory.strip_prefix("10.")?;
        let well_formed = all_digits(registrant, 4..=9)
            && !suffix.is_empty()
            && !raw.chars().any(char::is_whitespace)
            && !suffix
                .split('/')
                .any(|segment| matches!(segment, "." | ".."));
        well_formed.then(|| Self(raw.to_owned()))
    }

    /// Percent-encoded outside the RFC 3986 unreserved set and `/`.
    pub fn encoded(&self) -> String {
        percent_encode(&self.0, |c| is_unreserved(c) || c == '/', "%20")
    }

    pub fn url(&self) -> String {
        format!("https://doi.org/{}", self.encoded())
    }
}

/// A validated OpenAlex work identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkId(String);

impl WorkId {
    fn recognise(raw: &str) -> Option<Self> {
        let digits = raw.strip_prefix('W')?;
        all_digits(digits, 1..=usize::MAX).then(|| Self(raw.to_owned()))
    }

    /// Reads the identifier out of an `https://openalex.org/W…` URL or a bare
    /// `W…` form.
    pub fn from_openalex(raw: &str) -> Option<Self> {
        Self::recognise(
            raw.strip_prefix("https://openalex.org/").unwrap_or(raw),
        )
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn url(&self) -> String {
        format!("https://openalex.org/{}", self.0)
    }
}

/// What an OpenAlex lookup may address: a work, or a DOI OpenAlex resolves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenAlexId {
    Work(WorkId),
    Doi(Doi),
}

impl OpenAlexId {
    /// # Errors
    ///
    /// [`RequestError::MalformedOpenAlexId`] naming the rejected text.
    pub fn parse(raw: &str) -> Result<Self, RequestError> {
        let names_a_work =
            raw.starts_with('W') || raw.starts_with("https://openalex.org/");
        let recognised = if names_a_work {
            WorkId::from_openalex(raw).map(Self::Work)
        } else {
            let doi = raw
                .strip_prefix("doi:")
                .or_else(|| raw.strip_prefix("https://doi.org/"))
                .unwrap_or(raw);
            Doi::parse(doi).map(Self::Doi)
        };
        recognised
            .ok_or_else(|| RequestError::MalformedOpenAlexId(raw.to_owned()))
    }
}

/// Free text an OpenAlex `title_and_abstract.search` filter can carry: the
/// filter syntax's own `,`, `|`, `!` and `:` become spaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAlexQuery(String);

impl OpenAlexQuery {
    /// # Errors
    ///
    /// [`RequestError::EmptyQuery`] when nothing but filter syntax remains.
    pub fn parse(raw: &str) -> Result<Self, RequestError> {
        let words = raw
            .split(|c: char| {
                c.is_whitespace() || matches!(c, ',' | '|' | '!' | ':')
            })
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>();
        if words.is_empty() {
            return Err(RequestError::EmptyQuery);
        }
        Ok(Self(words.join(" ")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Free text translated into arXiv's query language: one `all:` clause per
/// word, joined by `AND`, with grouping, quoting and operator words dropped so
/// the result always parses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArxivQuery(String);

impl ArxivQuery {
    const OPERATORS: [&str; 3] = ["AND", "OR", "ANDNOT"];

    /// # Errors
    ///
    /// [`RequestError::EmptyQuery`] when no searchable word remains.
    pub fn parse(raw: &str) -> Result<Self, RequestError> {
        let clauses = raw
            .split(|c: char| {
                c.is_whitespace() || matches!(c, '(' | ')' | '"' | '\'')
            })
            .filter(|word| !word.is_empty() && !Self::OPERATORS.contains(word))
            .map(|word| format!("all:{word}"))
            .collect::<Vec<_>>();
        if clauses.is_empty() {
            return Err(RequestError::EmptyQuery);
        }
        Ok(Self(clauses.join(" AND ")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenAlexRequest {
    Search { query: OpenAlexQuery, limit: Limit },
    Lookup(OpenAlexId),
}

impl OpenAlexRequest {
    pub const fn verb(&self) -> Verb {
        match self {
            Self::Search { .. } => Verb::Search,
            Self::Lookup(_) => Verb::Lookup,
        }
    }

    pub const fn limit(&self) -> Limit {
        match self {
            Self::Search { limit, .. } => *limit,
            Self::Lookup(_) => Limit(1),
        }
    }

    pub fn upstream(
        &self,
        api: &Endpoint,
        key: Option<&ApiKey>,
    ) -> UpstreamRequest {
        let url = match self {
            Self::Search { query, limit } => Query::at(api, "/works")
                .encoded(
                    "filter",
                    &format!("title_and_abstract.search:{}", query.as_str()),
                )
                .encoded("per_page", &limit.get().to_string())
                .fixed("select", OPENALEX_SELECT),
            Self::Lookup(OpenAlexId::Work(work)) => {
                Query::at(api, &format!("/works/{}", work.as_str()))
                    .fixed("select", OPENALEX_SELECT)
            }
            Self::Lookup(OpenAlexId::Doi(doi)) => {
                Query::at(api, &format!("/works/doi:{}", doi.encoded()))
                    .fixed("select", OPENALEX_SELECT)
            }
        };
        UpstreamRequest {
            url: url.finish(),
            headers: vec![("User-Agent", USER_AGENT.to_owned())],
            bearer: key.map(|key| key.secret.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArxivRequest {
    Search { query: ArxivQuery, limit: Limit },
    Lookup(ArxivId),
}

impl ArxivRequest {
    pub const fn verb(&self) -> Verb {
        match self {
            Self::Search { .. } => Verb::Search,
            Self::Lookup(_) => Verb::Lookup,
        }
    }

    pub const fn limit(&self) -> Limit {
        match self {
            Self::Search { limit, .. } => *limit,
            Self::Lookup(_) => Limit(1),
        }
    }

    pub fn upstream(&self, api: &Endpoint) -> UpstreamRequest {
        let url = match self {
            Self::Search { query, limit } => Query::at(api, "/api/query")
                .encoded("search_query", query.as_str())
                .encoded("max_results", &limit.get().to_string())
                .fixed("sortBy", "relevance"),
            Self::Lookup(id) => Query::at(api, "/api/query")
                .encoded("id_list", id.unversioned()),
        };
        UpstreamRequest::arxiv(url)
    }
}

/// A validated request for one family, as a caller spells it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchRequest {
    OpenAlex(OpenAlexRequest),
    Arxiv(ArxivRequest),
}

impl FetchRequest {
    /// Validates every argument before any request, joining search terms with
    /// single spaces.
    ///
    /// # Errors
    ///
    /// A [`RequestError`] naming the first argument found invalid.
    pub fn parse(
        family: &str,
        verb: &str,
        terms: &[String],
        limit: Option<&str>,
    ) -> Result<Self, RequestError> {
        let family = Family::parse(family)?;
        let verb = Verb::parse(verb)?;
        let limit = limit.map_or_else(|| Ok(Limit::default()), Limit::parse)?;
        if terms.is_empty() {
            return Err(match verb {
                Verb::Search => RequestError::MissingQuery,
                Verb::Lookup => RequestError::MissingId,
            });
        }
        let text = terms.join(" ");
        Ok(match (family, verb) {
            (Family::OpenAlex, Verb::Search) => {
                Self::OpenAlex(OpenAlexRequest::Search {
                    query: OpenAlexQuery::parse(&text)?,
                    limit,
                })
            }
            (Family::OpenAlex, Verb::Lookup) => Self::OpenAlex(
                OpenAlexRequest::Lookup(OpenAlexId::parse(&text)?),
            ),
            (Family::Arxiv, Verb::Search) => {
                Self::Arxiv(ArxivRequest::Search {
                    query: ArxivQuery::parse(&text)?,
                    limit,
                })
            }
            (Family::Arxiv, Verb::Lookup) => {
                Self::Arxiv(ArxivRequest::Lookup(ArxivId::parse(&text)?))
            }
        })
    }

    pub const fn family(&self) -> Family {
        match self {
            Self::OpenAlex(_) => Family::OpenAlex,
            Self::Arxiv(_) => Family::Arxiv,
        }
    }

    pub const fn verb(&self) -> Verb {
        match self {
            Self::OpenAlex(request) => request.verb(),
            Self::Arxiv(request) => request.verb(),
        }
    }
}

/// The name of the credential rung that supplied a key, reported when the
/// key is rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySource(String);

impl KeySource {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl fmt::Display for KeySource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// An OpenAlex API key and where it came from.
#[derive(Clone)]
pub struct ApiKey {
    secret: String,
    source: KeySource,
}

impl ApiKey {
    pub const fn new(secret: String, source: KeySource) -> Self {
        Self { secret, source }
    }

    pub const fn source(&self) -> &KeySource {
        &self.source
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApiKey")
            .field("secret", &"<hidden>")
            .field("source", &self.source)
            .finish()
    }
}

/// The base URL of an upstream API, without a trailing slash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint(String);

impl Endpoint {
    pub fn new(base: &str) -> Self {
        Self(base.trim_end_matches('/').to_owned())
    }

    pub fn openalex() -> Self {
        Self::new("https://api.openalex.org")
    }

    pub fn arxiv_api() -> Self {
        Self::new("https://export.arxiv.org")
    }

    pub fn arxiv_oai() -> Self {
        Self::new("https://oaipmh.arxiv.org")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Everything a transport needs to send one `GET`, so it holds no
/// per-family knowledge.
#[derive(Clone, PartialEq, Eq)]
pub struct UpstreamRequest {
    url: String,
    headers: Vec<(&'static str, String)>,
    bearer: Option<String>,
}

impl UpstreamRequest {
    /// The OAI-PMH `arXivRaw` record that shows whether an entry's latest
    /// version was withdrawn.
    pub fn withdrawal_confirmation(oai: &Endpoint, id: &ArxivId) -> Self {
        Self::arxiv(
            Query::at(oai, "/oai")
                .fixed("verb", "GetRecord")
                .encoded(
                    "identifier",
                    &format!("oai:arXiv.org:{}", id.unversioned()),
                )
                .fixed("metadataPrefix", "arXivRaw"),
        )
    }

    fn arxiv(url: Query) -> Self {
        Self {
            url: url.finish(),
            headers: vec![
                ("User-Agent", USER_AGENT.to_owned()),
                ("Accept", ATOM.to_owned()),
            ],
            bearer: None,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn headers(&self) -> &[(&'static str, String)] {
        &self.headers
    }

    /// The token for an `Authorization: Bearer` header, if the request is
    /// keyed.
    pub fn bearer(&self) -> Option<&str> {
        self.bearer.as_deref()
    }
}

impl fmt::Debug for UpstreamRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UpstreamRequest")
            .field("url", &self.url)
            .field("headers", &self.headers)
            .field("bearer", &self.bearer.as_ref().map(|_| "<hidden>"))
            .finish()
    }
}

/// A URL whose query values can only be added through the encoder, except
/// for compile-time constants.
struct Query {
    url: String,
    separator: char,
}

impl Query {
    fn at(endpoint: &Endpoint, path: &str) -> Self {
        Self {
            url: format!("{}{path}", endpoint.as_str()),
            separator: '?',
        }
    }

    fn encoded(self, name: &'static str, value: &str) -> Self {
        let encoded = percent_encode(
            value,
            |c| is_unreserved(c) || matches!(c, ':' | '/'),
            "+",
        );
        self.append(name, &encoded)
    }

    fn fixed(self, name: &'static str, value: &'static str) -> Self {
        self.append(name, value)
    }

    fn append(mut self, name: &str, value: &str) -> Self {
        self.url.push(self.separator);
        self.url.push_str(name);
        self.url.push('=');
        self.url.push_str(value);
        self.separator = '&';
        self
    }

    fn finish(self) -> String {
        self.url
    }
}

const HEX: &[u8; 16] = b"0123456789ABCDEF";

const fn is_unreserved(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~')
}

fn percent_encode(
    text: &str,
    keep: impl Fn(char) -> bool,
    space: &str,
) -> String {
    let mut encoded = String::with_capacity(text.len());
    for c in text.chars() {
        if keep(c) {
            encoded.push(c);
        } else if c == ' ' {
            encoded.push_str(space);
        } else {
            let mut bytes = [0; 4];
            for byte in c.encode_utf8(&mut bytes).bytes() {
                encoded.push('%');
                encoded.push(char::from(HEX[usize::from(byte >> 4)]));
                encoded.push(char::from(HEX[usize::from(byte & 0x0F)]));
            }
        }
    }
    encoded
}
