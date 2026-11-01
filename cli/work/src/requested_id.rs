//! An ID a caller asks a new work item to take, admitted only in the exact
//! shape the configured pattern renders and only while no item holds it.

use std::fmt;
use std::path::Path;
use std::path::PathBuf;

use crate::draft_id::DraftId;
use crate::id_pattern::escaped;
use crate::id_pattern::IdKey;
use crate::id_pattern::IdParts;
use crate::id_pattern::PatternKeying;
use crate::id_scheme::WorkItemIdScheme;
use crate::identity::IdentityField;
use crate::retirement::relative_to;
use crate::work_item_files::identity_of;
use crate::work_item_files::WorkItemFile;
use crate::work_item_files::WorkItemListing;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestedId {
    text: String,
    parts: IdParts,
    scheme: WorkItemIdScheme,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proposal {
    Exact(RequestedId),
    OtherKey(RequestedId),
    Corrected { id: RequestedId, key_differs: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestedIdRefusal {
    WithProject {
        pattern: String,
        remedy: ProjectRemedy,
    },
    TrackerOwned {
        pattern: String,
    },
    DraftReserved {
        requested: String,
        shape: ExampleShape,
    },
    PatternMismatch {
        requested: String,
        shape: ExampleShape,
        correction: Option<String>,
    },
    Taken {
        requested: String,
        holder: PathBuf,
        holding: Holding,
        held: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectRemedy {
    WriteKeyIntoId {
        example: Option<String>,
        project_is_a_key: bool,
    },
    DropProject,
    DropBoth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleShape {
    pub pattern: String,
    pub description: String,
    pub first: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holding {
    Filename,
    Field(IdentityField),
}

impl RequestedId {
    /// `requested` as given, when it may name a new item.
    ///
    /// # Errors
    ///
    /// [`RequestedIdRefusal::WithProject`] when any `project` is given,
    /// [`RequestedIdRefusal::TrackerOwned`] under `{tracker}`,
    /// [`RequestedIdRefusal::DraftReserved`] for a draft ID, and
    /// [`RequestedIdRefusal::PatternMismatch`] for anything not in the
    /// pattern's exact shape.
    pub fn admit(
        requested: &str,
        project: Option<&str>,
        scheme: &WorkItemIdScheme,
    ) -> Result<Self, RequestedIdRefusal> {
        let pattern = scheme.pattern().as_str().to_owned();
        if let Some(project) = project {
            return Err(RequestedIdRefusal::WithProject {
                remedy: ProjectRemedy::for_request(requested, project, scheme),
                pattern,
            });
        }
        if scheme.pattern().is_tracker_owned() {
            return Err(RequestedIdRefusal::TrackerOwned { pattern });
        }
        if DraftId::parse(requested).is_some() {
            return Err(RequestedIdRefusal::DraftReserved {
                requested: requested.to_owned(),
                shape: ExampleShape::of(scheme),
            });
        }
        in_exact_shape(requested, scheme)
            .map(|parts| Self::of(requested.to_owned(), parts, scheme))
            .ok_or_else(|| RequestedIdRefusal::PatternMismatch {
                requested: requested.to_owned(),
                shape: ExampleShape::of(scheme),
                correction: correction_for(requested, scheme)
                    .map(|(corrected, _)| corrected),
            })
    }

    /// `requested` as given, or the one correction that puts it in the
    /// pattern's exact shape.
    ///
    /// # Errors
    ///
    /// The refusal [`Self::admit`] gives `requested` when no correction
    /// makes it exact.
    pub fn propose(
        requested: &str,
        scheme: &WorkItemIdScheme,
    ) -> Result<Proposal, RequestedIdRefusal> {
        match Self::admit(requested, None, scheme) {
            Ok(id) if id.key_differs() => Ok(Proposal::OtherKey(id)),
            Ok(id) => Ok(Proposal::Exact(id)),
            Err(refusal @ RequestedIdRefusal::PatternMismatch { .. }) => {
                let (corrected, parts) =
                    correction_for(requested, scheme).ok_or(refusal)?;
                let id = Self::of(corrected, parts, scheme);
                let key_differs = id.key_differs();
                Ok(Proposal::Corrected { id, key_differs })
            }
            Err(refusal) => Err(refusal),
        }
    }

    fn of(text: String, parts: IdParts, scheme: &WorkItemIdScheme) -> Self {
        Self {
            text,
            parts,
            scheme: scheme.clone(),
        }
    }

    fn key_differs(&self) -> bool {
        self.parts
            .key()
            .is_some_and(|key| !self.scheme.is_own_key(key))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// # Errors
    ///
    /// [`RequestedIdRefusal::Taken`] naming the first file, in path order,
    /// whose `id`, `aliases`, `external_id` or filename prefix holds this
    /// ID; an unreadable file holds it by its filename prefix alone.
    pub fn refuse_if_taken(
        &self,
        listing: &WorkItemListing,
    ) -> Result<(), RequestedIdRefusal> {
        let readable = listing
            .files
            .iter()
            .map(|file| (file.path.as_path(), Some(file)));
        let unreadable = listing
            .unreadable
            .iter()
            .map(|file| (file.path.as_path(), None));
        let mut candidates: Vec<_> = readable.chain(unreadable).collect();
        candidates.sort_by_key(|(path, _)| *path);
        let first_holder = candidates.into_iter().find_map(|(path, file)| {
            file.and_then(|file| self.field_holding(file))
                .or_else(|| self.filename_holding(path))
                .map(|(holding, held)| (path, holding, held))
        });
        match first_holder {
            None => Ok(()),
            Some((holder, holding, held)) => Err(RequestedIdRefusal::Taken {
                requested: self.text.clone(),
                holder: holder.to_path_buf(),
                holding,
                held,
            }),
        }
    }

    fn field_holding(&self, file: &WorkItemFile) -> Option<(Holding, String)> {
        let identity = identity_of(file)?;
        let holds = |value: &str| self.scheme.holds(value, &self.parts);
        let field = identity.fields_matching(holds).into_iter().next()?;
        let held = match field {
            IdentityField::Id => Some(identity.id),
            IdentityField::Alias => {
                identity.aliases.into_iter().find(|alias| holds(alias))
            }
            IdentityField::ExternalId => identity.external_id,
        }?;
        Some((Holding::Field(field), held))
    }

    fn filename_holding(&self, path: &Path) -> Option<(Holding, String)> {
        let filename = path.file_name()?.to_string_lossy();
        let (parts, slug_start) = self.scheme.read_held_prefix(&filename)?;
        parts
            .same_id(&self.parts)
            .then(|| (Holding::Filename, filename[..slug_start - 1].to_owned()))
    }
}

/// The only gate between a caller's string and the path a create writes:
/// ASCII in the pattern's exact shape, with a number of at least 1.
fn in_exact_shape(text: &str, scheme: &WorkItemIdScheme) -> Option<IdParts> {
    scheme
        .pattern()
        .read_exact(text)
        .filter(|parts| parts.number() >= 1)
}

/// The one correction `requested` may take: its reading rendered through
/// the pattern, keeping a carried key's case and otherwise taking the
/// scheme's key, kept only when the rendering is in the exact shape.
fn correction_for(
    requested: &str,
    scheme: &WorkItemIdScheme,
) -> Option<(String, IdParts)> {
    let reading = scheme.pattern().read_correctable(requested)?;
    let carried = reading.key().and_then(|key| IdKey::parse(key).ok());
    let key = carried.as_ref().or_else(|| scheme.key());
    let rendered = scheme.pattern().render(key, reading.number()).ok()?;
    let parts = in_exact_shape(&rendered, scheme)?;
    Some((rendered, parts))
}

impl ProjectRemedy {
    fn for_request(
        requested: &str,
        project: &str,
        scheme: &WorkItemIdScheme,
    ) -> Self {
        match scheme.pattern().keying() {
            PatternKeying::Tracker => Self::DropBoth,
            PatternKeying::Keyless => Self::DropProject,
            PatternKeying::Keyed => {
                let project_key = IdKey::parse(project).ok();
                let number = scheme
                    .pattern()
                    .read_correctable(requested)
                    .map_or(1, |parts| parts.number().max(1));
                let key = project_key.as_ref().or_else(|| scheme.key());
                Self::WriteKeyIntoId {
                    example: scheme.pattern().render(key, number).ok(),
                    project_is_a_key: project_key.is_some(),
                }
            }
        }
    }

    fn text(&self, pattern: &str) -> String {
        match self {
            Self::WriteKeyIntoId {
                example,
                project_is_a_key,
            } => {
                let example = example
                    .as_deref()
                    .map(|example| format!(", as in '{example}',"))
                    .unwrap_or_default();
                let allocate = if *project_is_a_key {
                    "; or drop --id to allocate under --project"
                } else {
                    ""
                };
                format!(
                    "Write the key into the ID{example} and drop \
                     --project{allocate}."
                )
            }
            Self::DropProject => {
                format!("Pattern '{pattern}' has no {{key}}. Drop --project.")
            }
            Self::DropBoth => {
                format!("Pattern '{pattern}' assigns every ID. Drop both.")
            }
        }
    }
}

impl ExampleShape {
    fn of(scheme: &WorkItemIdScheme) -> Self {
        Self {
            pattern: scheme.pattern().as_str().to_owned(),
            description: scheme.pattern().describe(),
            first: scheme.pattern().render(scheme.key(), 1).ok(),
        }
    }

    fn example_clause(&self) -> String {
        self.first
            .as_deref()
            .map(|first| format!(", shaped like '{first}'"))
            .unwrap_or_default()
    }
}

impl Holding {
    const fn named(self) -> &'static str {
        match self {
            Self::Filename => "its filename prefix",
            Self::Field(IdentityField::Id) => "its id",
            Self::Field(IdentityField::Alias) => "an alias",
            Self::Field(IdentityField::ExternalId) => "its external_id",
        }
    }
}

impl RequestedIdRefusal {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::WithProject { .. } => "E_ID_WITH_PROJECT",
            Self::TrackerOwned { .. } => "E_ID_TRACKER_OWNED",
            Self::DraftReserved { .. } => "E_ID_DRAFT_RESERVED",
            Self::PatternMismatch { .. } => "E_ID_PATTERN_MISMATCH",
            Self::Taken { .. } => "E_ID_TAKEN",
        }
    }

    /// The refusal naming its holder relative to `root`, for display.
    #[must_use]
    pub fn relative_to(self, root: &Path) -> Self {
        match self {
            Self::Taken {
                requested,
                holder,
                holding,
                held,
            } => Self::Taken {
                requested,
                holder: relative_to(holder, root),
                holding,
                held,
            },
            other => other,
        }
    }

    fn what(&self) -> String {
        match self {
            Self::WithProject { .. } => {
                "--id cannot be combined with --project".to_owned()
            }
            Self::TrackerOwned { pattern } => format!(
                "pattern '{pattern}' lets the tracker assign every ID, so \
                 none can be requested"
            ),
            Self::DraftReserved { requested, .. } => format!(
                "'{}' has the draft- form reserved for provisional drafts",
                escaped(requested)
            ),
            Self::PatternMismatch {
                requested, shape, ..
            } => format!(
                "'{}' is not in the exact shape of pattern '{}'",
                escaped(requested),
                shape.pattern
            ),
            Self::Taken {
                requested,
                holder,
                holding,
                held,
            } => format!(
                "'{}' is already held by {} as {} '{}'",
                escaped(requested),
                escaped(&holder.to_string_lossy()),
                holding.named(),
                escaped(held)
            ),
        }
    }

    fn remedy(&self) -> String {
        match self {
            Self::WithProject { pattern, remedy } => remedy.text(pattern),
            Self::TrackerOwned { .. } => "Create the item without one: it is \
                 saved as a draft and takes the tracker's key when pushed."
                .to_owned(),
            Self::DraftReserved { shape, .. } => format!(
                "Choose an ID in the shape of pattern '{}'{}.",
                shape.pattern,
                shape.example_clause()
            ),
            Self::PatternMismatch {
                correction: Some(corrected),
                ..
            } => format!("Did you mean '{corrected}'?"),
            Self::PatternMismatch {
                shape,
                correction: None,
                ..
            } => format!(
                "Write it as the pattern renders it{}: {}.",
                shape.example_clause(),
                shape.description
            ),
            Self::Taken { .. } => "Choose another ID, or let the next number \
                 be allocated."
                .to_owned(),
        }
    }
}

impl fmt::Display for RequestedIdRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {}. {}",
            self.code(),
            self.what(),
            self.remedy()
        )
    }
}

impl std::error::Error for RequestedIdRefusal {}

#[cfg(test)]
// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::literal_string_with_formatting_args
)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;
    use std::path::PathBuf;

    use super::ExampleShape;
    use super::Holding;
    use super::ProjectRemedy;
    use super::Proposal;
    use super::RequestedId;
    use super::RequestedIdRefusal;
    use crate::id_pattern::IdKey;
    use crate::id_pattern::IdPattern;
    use crate::id_scheme::WorkItemIdScheme;
    use crate::identity::IdentityField;
    use crate::work_item_files::ReadFailure;
    use crate::work_item_files::UnreadableFile;
    use crate::work_item_files::WorkItemFile;
    use crate::work_item_files::WorkItemListing;

    fn scheme(pattern: &str, key: Option<&str>) -> WorkItemIdScheme {
        WorkItemIdScheme::new(
            IdPattern::parse(pattern).expect("a well-formed pattern"),
            key.map(|key| IdKey::parse(key).expect("a well-formed key")),
        )
    }

    fn numeric() -> WorkItemIdScheme {
        scheme("{number:04d}", None)
    }

    fn keyed() -> WorkItemIdScheme {
        scheme("{key}-{number:04d}", Some("PP"))
    }

    fn tracker() -> WorkItemIdScheme {
        scheme("{tracker}", None)
    }

    fn admitted(requested: &str, under: &WorkItemIdScheme) -> RequestedId {
        RequestedId::admit(requested, None, under)
            .unwrap_or_else(|refusal| unreachable!("{requested}: {refusal}"))
    }

    fn refusal(
        requested: &str,
        project: Option<&str>,
        under: &WorkItemIdScheme,
    ) -> RequestedIdRefusal {
        RequestedId::admit(requested, project, under)
            .expect_err(&format!("{requested} is refused"))
    }

    fn refused_as(
        requested: &str,
        project: Option<&str>,
        under: &WorkItemIdScheme,
    ) -> &'static str {
        refusal(requested, project, under).code()
    }

    fn file(path: &str, frontmatter: &str) -> WorkItemFile {
        let content = if frontmatter.is_empty() {
            String::new()
        } else {
            format!("---\n{frontmatter}---\n\n# Title\n")
        };
        WorkItemFile {
            path: PathBuf::from(path),
            content,
        }
    }

    fn unreadable_at(path: &str) -> UnreadableFile {
        UnreadableFile {
            path: PathBuf::from(path),
            reason: ReadFailure::Undecodable,
        }
    }

    fn named(filename: &str) -> WorkItemFile {
        file(&format!("meta/work/{filename}"), "")
    }

    fn holding(frontmatter: &str) -> WorkItemFile {
        file("meta/work/0007-holder.md", frontmatter)
    }

    fn listing(files: Vec<WorkItemFile>) -> WorkItemListing {
        WorkItemListing {
            files,
            unreadable: Vec::new(),
        }
    }

    fn holder_of(
        requested: &str,
        under: &WorkItemIdScheme,
        listing: &WorkItemListing,
    ) -> Option<(PathBuf, Holding, String)> {
        match admitted(requested, under).refuse_if_taken(listing) {
            Ok(()) => None,
            Err(RequestedIdRefusal::Taken {
                holder,
                holding,
                held,
                ..
            }) => Some((holder, holding, held)),
            Err(other) => unreachable!("{requested}: {other}"),
        }
    }

    fn is_taken(
        requested: &str,
        under: &WorkItemIdScheme,
        listing: &WorkItemListing,
    ) -> bool {
        holder_of(requested, under, listing).is_some()
    }

    fn message(refusal: &RequestedIdRefusal) -> String {
        refusal.to_string()
    }

    #[test]
    fn an_id_with_a_project_is_refused_under_every_pattern() {
        for under in [numeric(), keyed(), tracker()] {
            for project in ["XY", "1X", ""] {
                assert_eq!(
                    refused_as("0400", Some(project), &under),
                    "E_ID_WITH_PROJECT",
                    "{project} under {}",
                    under.pattern().as_str()
                );
            }
        }
    }

    #[test]
    fn an_id_under_tracker_is_refused() {
        for requested in ["0400", "PP-0400", "draft-k7mq3x"] {
            assert_eq!(
                refused_as(requested, None, &tracker()),
                "E_ID_TRACKER_OWNED",
                "{requested}"
            );
        }
    }

    #[test]
    fn a_draft_id_is_reserved_under_every_local_pattern() {
        for under in [numeric(), keyed()] {
            for requested in ["draft-k7mq3x", "DRAFT-K7MQ3X"] {
                assert_eq!(
                    refused_as(requested, None, &under),
                    "E_ID_DRAFT_RESERVED",
                    "{requested}"
                );
            }
        }
    }

    #[test]
    fn an_id_not_in_the_patterns_exact_shape_is_a_mismatch() {
        let cases = [
            (
                numeric(),
                vec![
                    "400",
                    "04000",
                    "10000",
                    "0000",
                    "XY-0400",
                    "foo",
                    "",
                    "draft-k7mq3",
                    "draft-k7mq3xy",
                    "draft-k7mq3u",
                    "../0400",
                    "sub/0400",
                    "0400\n",
                    " 0400",
                    "0400 ",
                    "\u{FF10}\u{FF14}\u{FF10}\u{FF10}",
                    "\u{0660}\u{0664}\u{0660}\u{0660}",
                ],
            ),
            (
                keyed(),
                vec!["0400", "1X-0400", "X_Y-0400", "XY400", "XY-400"],
            ),
            (scheme("{number:019d}", None), vec!["99999999999999999999"]),
        ];
        for (under, requests) in cases {
            for requested in requests {
                assert_eq!(
                    refused_as(requested, None, &under),
                    "E_ID_PATTERN_MISMATCH",
                    "{requested:?} under {}",
                    under.pattern().as_str()
                );
            }
        }
    }

    #[test]
    fn an_id_in_the_patterns_exact_shape_is_admitted_as_given() {
        let cases = [
            (numeric(), vec!["0001", "0400", "9999"]),
            (
                keyed(),
                vec![
                    "PP-0400", "XY-0400", "xy-0400", "Xy-0400", "X-0400",
                    "X1-0400",
                ],
            ),
            (
                scheme("{key}-{number:06d}", Some("draft")),
                vec!["draft-040000"],
            ),
            (
                scheme("{project}-{number:04d}", Some("PP")),
                vec!["PP-0400"],
            ),
        ];
        for (under, requests) in cases {
            for requested in requests {
                assert_eq!(admitted(requested, &under).as_str(), requested);
            }
        }
    }

    #[test]
    fn a_bare_number_token_requires_four_digits() {
        let under = scheme("{number}", None);
        assert_eq!(admitted("0400", &under).as_str(), "0400");
        for requested in ["400", "00400"] {
            assert_eq!(
                refused_as(requested, None, &under),
                "E_ID_PATTERN_MISMATCH"
            );
        }
    }

    #[test]
    fn literal_text_in_the_pattern_is_part_of_the_exact_shape() {
        let under = scheme("v{number:03d}", None);
        assert_eq!(admitted("v001", &under).as_str(), "v001");
        for requested in ["001", "V001"] {
            assert_eq!(
                refused_as(requested, None, &under),
                "E_ID_PATTERN_MISMATCH"
            );
        }
    }

    #[test]
    fn a_key_before_an_alphanumeric_literal_is_admitted() {
        let under = scheme("{key}v{number:03d}", Some("PP"));
        for requested in ["PPv001", "xyv001"] {
            assert_eq!(admitted(requested, &under).as_str(), requested);
        }
    }

    enum Proposed {
        Exact(String),
        OtherKey(String),
        Corrected(String, bool),
    }

    fn proposed(requested: &str, under: &WorkItemIdScheme) -> Proposed {
        match RequestedId::propose(requested, under) {
            Ok(Proposal::Exact(id)) => Proposed::Exact(id.as_str().to_owned()),
            Ok(Proposal::OtherKey(id)) => {
                Proposed::OtherKey(id.as_str().to_owned())
            }
            Ok(Proposal::Corrected { id, key_differs }) => {
                Proposed::Corrected(id.as_str().to_owned(), key_differs)
            }
            Err(refusal) => unreachable!("{requested}: {refusal}"),
        }
    }

    fn proposal_refused_as(
        requested: &str,
        under: &WorkItemIdScheme,
    ) -> RequestedIdRefusal {
        match RequestedId::propose(requested, under) {
            Err(refusal) => refusal,
            Ok(_) => unreachable!("{requested} has a proposal"),
        }
    }

    #[test]
    fn an_exact_request_is_proposed_as_given() {
        for (requested, under) in [
            ("0400", numeric()),
            ("PP-0400", keyed()),
            ("pp-0400", keyed()),
        ] {
            assert!(
                matches!(
                    proposed(requested, &under),
                    Proposed::Exact(id) if id == requested
                ),
                "{requested}"
            );
        }
    }

    #[test]
    fn an_exact_request_under_another_key_is_proposed_as_other_key() {
        for requested in ["XY-0400", "xy-0400"] {
            assert!(
                matches!(
                    proposed(requested, &keyed()),
                    Proposed::OtherKey(id) if id == requested
                ),
                "{requested}"
            );
        }
    }

    fn corrected(requested: &str, under: &WorkItemIdScheme) -> (String, bool) {
        match proposed(requested, under) {
            Proposed::Corrected(id, key_differs) => (id, key_differs),
            Proposed::Exact(id) | Proposed::OtherKey(id) => {
                unreachable!("{requested} proposed {id} as given")
            }
        }
    }

    #[test]
    fn a_short_number_is_padded_to_the_patterns_width() {
        assert_eq!(corrected("400", &numeric()), ("0400".to_owned(), false));
        assert_eq!(corrected("1", &numeric()), ("0001".to_owned(), false));
    }

    #[test]
    fn a_keyless_request_takes_the_configured_key() {
        for requested in ["400", "0400"] {
            assert_eq!(
                corrected(requested, &keyed()),
                ("PP-0400".to_owned(), false),
                "{requested}"
            );
        }
    }

    #[test]
    fn a_carried_key_is_kept_in_its_case() {
        assert_eq!(corrected("XY-400", &keyed()), ("XY-0400".to_owned(), true));
        assert_eq!(corrected("xy-400", &keyed()), ("xy-0400".to_owned(), true));
        assert_eq!(corrected("400", &keyed()), ("PP-0400".to_owned(), false));
    }

    #[test]
    fn literal_text_is_never_added_or_recased() {
        let under = scheme("v{number:03d}", None);
        assert_eq!(corrected("v1", &under), ("v001".to_owned(), false));
        for requested in ["1", "V1"] {
            assert_eq!(
                proposal_refused_as(requested, &under).code(),
                "E_ID_PATTERN_MISMATCH",
                "{requested}"
            );
        }
    }

    #[test]
    fn a_request_no_correction_makes_exact_is_refused_as_given() {
        let mismatches = [
            ("0", numeric()),
            ("10000", numeric()),
            ("04000", numeric()),
            ("XY-400", numeric()),
            ("foo", numeric()),
            ("XY400", keyed()),
        ];
        for (requested, under) in mismatches {
            let refusal = proposal_refused_as(requested, &under);
            assert!(
                matches!(
                    &refusal,
                    RequestedIdRefusal::PatternMismatch { requested: named, .. }
                        if named == requested
                ),
                "{requested}: {refusal:?}"
            );
        }
        assert_eq!(
            proposal_refused_as("draft-k7mq3x", &numeric()).code(),
            "E_ID_DRAFT_RESERVED"
        );
        assert_eq!(
            proposal_refused_as("0400", &tracker()).code(),
            "E_ID_TRACKER_OWNED"
        );
    }

    #[test]
    fn nothing_held_leaves_the_id_untaken() {
        assert!(!is_taken("0400", &numeric(), &WorkItemListing::default()));
    }

    #[test]
    fn an_id_held_by_a_filename_under_any_slug_is_taken() {
        let held = listing(vec![named("0400-other.md")]);
        assert_eq!(
            holder_of("0400", &numeric(), &held),
            Some((
                PathBuf::from("meta/work/0400-other.md"),
                Holding::Filename,
                "0400".to_owned()
            ))
        );
    }

    #[test]
    fn an_id_held_as_an_id_alias_or_external_id_is_taken() {
        let cases = [
            ("id: \"0400\"\n", IdentityField::Id),
            ("id: \"0007\"\naliases: [\"0400\"]\n", IdentityField::Alias),
            (
                "id: \"0007\"\nexternal_id: \"0400\"\n",
                IdentityField::ExternalId,
            ),
        ];
        for (frontmatter, field) in cases {
            assert_eq!(
                holder_of(
                    "0400",
                    &numeric(),
                    &listing(vec![holding(frontmatter)])
                ),
                Some((
                    PathBuf::from("meta/work/0007-holder.md"),
                    Holding::Field(field),
                    "0400".to_owned()
                )),
                "{frontmatter}"
            );
        }
    }

    #[test]
    fn a_held_id_with_an_equal_number_at_any_width_takes_it() {
        for held in [
            listing(vec![holding("id: \"400\"\n")]),
            listing(vec![named("400-x.md")]),
            listing(vec![named("00400-x.md")]),
        ] {
            assert!(is_taken("0400", &numeric(), &held), "{held:?}");
        }
    }

    #[test]
    fn key_case_is_ignored_when_comparing_held_ids() {
        assert!(is_taken(
            "xy-0400",
            &keyed(),
            &listing(vec![holding("id: \"XY-0400\"\n")])
        ));
        for held in [
            listing(vec![named("xy-0400-x.md")]),
            listing(vec![holding("id: \"PP-0007\"\naliases: [\"xy-0400\"]\n")]),
            listing(vec![holding(
                "id: \"PP-0007\"\nexternal_id: \"xy-0400\"\n",
            )]),
        ] {
            assert!(is_taken("XY-0400", &keyed(), &held), "{held:?}");
        }
    }

    #[test]
    fn a_different_key_makes_a_different_id() {
        let held = listing(vec![holding("id: \"PP-0400\"\n")]);
        assert!(!is_taken("XY-0400", &keyed(), &held));
    }

    #[test]
    fn a_tracker_key_held_as_external_id_takes_the_local_id_with_its_key_and_number(
    ) {
        let held = listing(vec![holding(
            "id: \"PP-0007\"\nexternal_id: \"PP-905\"\n",
        )]);
        assert!(is_taken("PP-0905", &keyed(), &held));
    }

    #[test]
    fn an_unparseable_held_value_never_takes_a_requested_id() {
        let held = listing(vec![holding(
            "id: \"PP-0007\"\naliases: [\"draft-k7mq3x\", \"PP\u{2013}0400\"]\n",
        )]);
        for requested in ["PP-0400", "XY-0400"] {
            assert!(!is_taken(requested, &keyed(), &held), "{requested}");
        }
    }

    #[test]
    fn a_legacy_number_takes_the_configured_keys_id() {
        let legacy_filename = listing(vec![named("0400-x.md")]);
        let unreadable = WorkItemListing {
            files: Vec::new(),
            unreadable: vec![unreadable_at("meta/work/0400-y.md")],
        };
        let legacy_id = listing(vec![holding("id: \"0400\"\n")]);
        for held in [legacy_filename, unreadable, legacy_id] {
            let holder = holder_of("PP-0400", &keyed(), &held);
            assert_eq!(
                holder.map(|(_, _, value)| value).as_deref(),
                Some("0400"),
                "{held:?}"
            );
            assert!(!is_taken("XY-0400", &keyed(), &held), "{held:?}");
        }
    }

    #[test]
    fn a_filename_digit_run_too_long_for_u64_leaves_the_id_untaken() {
        let held = listing(vec![named("99999999999999999999-x.md")]);
        assert!(!is_taken("0400", &numeric(), &held));
    }

    #[test]
    fn a_draft_file_is_read_like_any_other() {
        let held = listing(vec![file(
            "meta/work/drafts/draft-k7mq3x-x.md",
            "id: \"0400\"\n",
        )]);
        assert!(is_taken("0400", &numeric(), &held));
    }

    #[test]
    fn an_unreadable_file_takes_its_id_by_filename() {
        let held = WorkItemListing {
            files: Vec::new(),
            unreadable: vec![unreadable_at("meta/work/0400-x.md")],
        };
        assert_eq!(
            holder_of("0400", &numeric(), &held),
            Some((
                PathBuf::from("meta/work/0400-x.md"),
                Holding::Filename,
                "0400".to_owned()
            ))
        );
    }

    #[test]
    fn the_first_holder_in_path_order_is_reported() {
        let held = WorkItemListing {
            files: vec![file("meta/work/0500-b.md", "id: \"0400\"\n")],
            unreadable: vec![unreadable_at("meta/work/0400-a.md")],
        };
        assert_eq!(
            holder_of("0400", &numeric(), &held).map(|(path, ..)| path),
            Some(PathBuf::from("meta/work/0400-a.md"))
        );
    }

    #[test]
    fn a_deprecated_project_token_compares_held_ids_by_key_and_number() {
        let under = scheme("{project}-{number:04d}", Some("PP"));
        let held = listing(vec![named("pp-0400-x.md")]);
        assert!(is_taken("PP-0400", &under, &held));
    }

    fn shape() -> ExampleShape {
        ExampleShape {
            pattern: "{number:04d}".to_owned(),
            description: "a 4-digit number from 0001 to 9999, with no key"
                .to_owned(),
            first: Some("0001".to_owned()),
        }
    }

    fn every_refusal() -> Vec<RequestedIdRefusal> {
        vec![
            RequestedIdRefusal::WithProject {
                pattern: "{number:04d}".to_owned(),
                remedy: ProjectRemedy::DropProject,
            },
            RequestedIdRefusal::TrackerOwned {
                pattern: "{tracker}".to_owned(),
            },
            RequestedIdRefusal::DraftReserved {
                requested: "draft-k7mq3x".to_owned(),
                shape: shape(),
            },
            RequestedIdRefusal::PatternMismatch {
                requested: "foo".to_owned(),
                shape: shape(),
                correction: None,
            },
            RequestedIdRefusal::Taken {
                requested: "0400".to_owned(),
                holder: PathBuf::from("meta/work/0400-x.md"),
                holding: Holding::Filename,
                held: "0400".to_owned(),
            },
        ]
    }

    /// Fails to compile when a variant is added, so `every_refusal` must
    /// gain it.
    const fn covered(refusal: &RequestedIdRefusal) {
        match refusal {
            RequestedIdRefusal::WithProject { .. }
            | RequestedIdRefusal::TrackerOwned { .. }
            | RequestedIdRefusal::DraftReserved { .. }
            | RequestedIdRefusal::PatternMismatch { .. }
            | RequestedIdRefusal::Taken { .. } => {}
        }
    }

    #[test]
    fn every_refusal_has_a_distinct_code() {
        let refusals = every_refusal();
        let codes: BTreeSet<&str> = refusals
            .iter()
            .map(|refusal| {
                covered(refusal);
                refusal.code()
            })
            .collect();
        assert_eq!(
            codes,
            BTreeSet::from([
                "E_ID_DRAFT_RESERVED",
                "E_ID_PATTERN_MISMATCH",
                "E_ID_TAKEN",
                "E_ID_TRACKER_OWNED",
                "E_ID_WITH_PROJECT",
            ])
        );
    }

    #[test]
    fn each_refusal_message_starts_with_its_code_and_names_its_cause() {
        for refusal in every_refusal() {
            let text = message(&refusal);
            assert!(
                text.starts_with(&format!("{}: ", refusal.code())),
                "{text}"
            );
        }
        let with_project = message(&refusal("0400", Some("XY"), &numeric()));
        assert!(with_project.contains("--id"), "{with_project}");
        assert!(with_project.contains("--project"), "{with_project}");
        assert!(
            message(&refusal("0400", None, &tracker())).contains("'{tracker}'")
        );
        let mismatch = message(&refusal("foo", None, &keyed()));
        assert!(mismatch.contains("'{key}-{number:04d}'"), "{mismatch}");
        let draft = message(&refusal("draft-k7mq3x", None, &keyed()));
        assert!(draft.contains("draft-"), "{draft}");
        assert!(draft.contains("'{key}-{number:04d}'"), "{draft}");
        let taken = admitted("0400", &numeric())
            .refuse_if_taken(&listing(vec![named("0400-x.md")]))
            .unwrap_err();
        assert!(message(&taken).contains("meta/work/0400-x.md"));
    }

    #[test]
    fn a_tracker_refusal_explains_how_to_create_the_item() {
        assert_eq!(
            message(&refusal("0400", None, &tracker())),
            "E_ID_TRACKER_OWNED: pattern '{tracker}' lets the tracker assign \
             every ID, so none can be requested. Create the item without \
             one: it is saved as a draft and takes the tracker's key when \
             pushed."
        );
    }

    #[test]
    fn a_taken_refusal_says_how_the_holder_holds_the_id() {
        let by_external_id = admitted("PP-0905", &keyed())
            .refuse_if_taken(&listing(vec![holding(
                "id: \"PP-0007\"\nexternal_id: \"PP-905\"\n",
            )]))
            .unwrap_err();
        assert!(
            message(&by_external_id).contains("as its external_id 'PP-905'"),
            "{by_external_id}"
        );
        let by_filename = admitted("0400", &numeric())
            .refuse_if_taken(&listing(vec![named("400-x.md")]))
            .unwrap_err();
        assert!(
            message(&by_filename).contains("as its filename prefix '400'"),
            "{by_filename}"
        );
    }

    fn remedy(
        requested: &str,
        project: &str,
        under: &WorkItemIdScheme,
    ) -> ProjectRemedy {
        match refusal(requested, Some(project), under) {
            RequestedIdRefusal::WithProject { remedy, .. } => remedy,
            other => unreachable!("{requested} --project {project}: {other}"),
        }
    }

    fn written_into(example: &str, project_is_a_key: bool) -> ProjectRemedy {
        ProjectRemedy::WriteKeyIntoId {
            example: Some(example.to_owned()),
            project_is_a_key,
        }
    }

    #[test]
    fn a_project_refusal_offers_only_remedies_the_pattern_allows() {
        assert_eq!(
            remedy("XY-0400", "XY", &keyed()),
            written_into("XY-0400", true)
        );
        assert_eq!(
            remedy("0400", "XY", &keyed()),
            written_into("XY-0400", true)
        );
        assert_eq!(
            remedy("0400", "1X", &keyed()),
            written_into("PP-0400", false)
        );
        assert_eq!(remedy("0", "XY", &keyed()), written_into("XY-0001", true));

        let allocating = message(&refusal("0400", Some("XY"), &keyed()));
        assert!(allocating.contains("'XY-0400'"), "{allocating}");
        assert!(
            allocating.contains("drop --id to allocate under --project"),
            "{allocating}"
        );
        let not_a_key = message(&refusal("0400", Some("1X"), &keyed()));
        assert!(!not_a_key.contains("allocate"), "{not_a_key}");

        assert_eq!(
            remedy("0400", "XY", &numeric()),
            ProjectRemedy::DropProject
        );
        assert!(message(&refusal("0400", Some("XY"), &numeric()))
            .contains("has no {key}"));
        assert_eq!(remedy("0400", "XY", &tracker()), ProjectRemedy::DropBoth);
        assert!(message(&refusal("0400", Some("XY"), &tracker()))
            .contains("{tracker}"));
    }

    #[test]
    fn a_shape_refusal_states_an_example() {
        let mismatch = message(&refusal("foo", None, &numeric()));
        assert!(mismatch.contains("shaped like '0001'"), "{mismatch}");
        let draft = message(&refusal("draft-k7mq3x", None, &numeric()));
        assert!(draft.contains("'{number:04d}'"), "{draft}");
        assert!(draft.contains("shaped like '0001'"), "{draft}");
        let keyed_mismatch = message(&refusal("foo", None, &keyed()));
        assert!(
            keyed_mismatch.contains("shaped like 'PP-0001'"),
            "{keyed_mismatch}"
        );
    }

    #[test]
    fn a_mismatch_with_no_key_in_the_pattern_describes_none() {
        let text = message(&refusal("XY-400", None, &numeric()));
        assert!(text.contains("with no key"), "{text}");
        assert!(!text.contains("a key of"), "{text}");
    }

    #[test]
    fn a_correctable_mismatch_names_its_correction() {
        let text = message(&refusal("400", None, &numeric()));
        assert!(text.contains("Did you mean '0400'?"), "{text}");
    }

    #[test]
    fn no_refusal_remedy_names_the_id_flag() {
        let refusals = [
            refusal("0400", None, &tracker()),
            refusal("draft-k7mq3x", None, &numeric()),
            refusal("foo", None, &numeric()),
            refusal("400", None, &numeric()),
            admitted("0400", &numeric())
                .refuse_if_taken(&listing(vec![named("0400-x.md")]))
                .unwrap_err(),
        ];
        for refusal in refusals {
            let text = message(&refusal);
            let (_, after_first_sentence) =
                text.split_once(". ").unwrap_or((&text, ""));
            assert!(!after_first_sentence.contains("--id"), "{text}");
        }
    }

    #[test]
    fn a_refusal_renders_its_holder_relative_to_the_root() {
        let root = Path::new("/repo");
        let held = listing(vec![file("/repo/meta/work/0400-x.md", "")]);
        let taken = admitted("0400", &numeric())
            .refuse_if_taken(&held)
            .unwrap_err()
            .relative_to(root);
        let text = message(&taken);
        assert!(text.contains(" meta/work/0400-x.md "), "{text}");
        assert!(!text.contains("/repo"), "{text}");

        let hostile = listing(vec![named("0400-a\u{1b}b.md")]);
        let text = message(
            &admitted("0400", &numeric())
                .refuse_if_taken(&hostile)
                .unwrap_err(),
        );
        assert!(text.contains("U+001B"), "{text}");
        assert!(!text.contains('\u{1b}'), "{text:?}");
    }

    #[test]
    fn the_shape_check_and_the_correction_terminate() {
        for requested in ["0", "0000"] {
            for refusal in [
                refusal(requested, None, &numeric()),
                proposal_refused_as(requested, &numeric()),
            ] {
                assert!(
                    matches!(
                        refusal,
                        RequestedIdRefusal::PatternMismatch {
                            correction: None,
                            ..
                        }
                    ),
                    "{requested}: {refusal:?}"
                );
            }
        }
    }

    #[test]
    fn a_requested_id_is_shown_escaped() {
        for requested in ["04\u{1b}00", "0400\n"] {
            let text = message(&refusal(requested, None, &numeric()));
            assert!(
                text.contains("U+001B") || text.contains("U+000A"),
                "{text:?}"
            );
            assert!(!text.contains(['\u{1b}', '\n']), "{text:?}");
        }
    }

    #[test]
    fn a_missing_key_omits_the_example() {
        let keyless = scheme("{key}-{number:04d}", None);
        for requested in ["foo", "draft-k7mq3x"] {
            let text = message(&refusal(requested, None, &keyless));
            assert!(!text.contains("shaped like"), "{text}");
        }
    }
}
