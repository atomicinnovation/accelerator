//! `ShortcutClient`: the `RemoteTracker` port over Shortcut's REST API v4.
//!
//! Construction reads the committed catalogue and makes no request. Every
//! entity is reported under the `ExternalId` it was asked for, so a requested
//! `SC-12` comes back as `SC-12` and a partition stays total.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;

use config::consent::Notice;
use config::consent::Refusal;
use config::credentials::CredentialContext;
use reqwest::Method;
use reqwest::Url;
use serde_json::json;
use serde_json::Value;
use tracker::BodyDialect;
use tracker::Ceiling;
use tracker::Completeness;
use tracker::CreatePreview;
use tracker::Discovery;
use tracker::EntityScope;
use tracker::ExternalId;
use tracker::FetchOutcome;
use tracker::FieldResolution;
use tracker::Located;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::ScopeError;
use tracker::SearchScope;
use tracker::TrackerError;
use tracker::ValidationOutcome;
use tracker::VisibleEntity;
use tracker_support::ClockJitter;
use tracker_support::SystemSleeper;
use tracker_support::TransportConfig;

use crate::auth::resolve_credentials;
use crate::cache::ShortcutCache;
use crate::cache::SystemFilesystem;
use crate::catalogue::Catalogue;
use crate::catalogue::StateType;
use crate::catalogue::Team;
use crate::catalogue::TeamLookupError;
use crate::classify::classify;
use crate::classify::explain;
use crate::classify::Failure;
use crate::classify::Operation;
use crate::classify::Outcome;
use crate::error::ClientError;
use crate::filter::SearchFilters;
use crate::new_entity::NewEntity;
use crate::paging::collect;
use crate::paging::Cut;
use crate::paging::PageError;
use crate::paths::Paths;
use crate::paths::PRODUCTION_BASE;
use crate::shortcut_key::Collection;
use crate::shortcut_key::ShortcutKey;
use crate::team_key::TeamIdentity;
use crate::team_key::TeamKeys;
use crate::transport::Deadline;
use crate::transport::Delivery;
use crate::transport::Received;
use crate::transport::Resend;
use crate::transport::Transport;
use crate::update::UpdateFields;
use crate::workspace::whoami;

const KEYED_CHUNK: usize = 100;
const PAGE_LIMIT: &str = "100";
const SHOW_FIELDS: &str = "id,name,description,updated_at,app_url";
const STAMP_FIELDS: &str = "id,updated_at";

pub struct ShortcutClient {
    transport: Transport,
    paths: Paths,
    catalogue: Catalogue,
    team_key: Option<String>,
    scopes: RefCell<Option<Vec<String>>>,
    refusals: Vec<Refusal>,
    notice: Option<Notice>,
}

impl ShortcutClient {
    #[must_use]
    pub fn new(
        transport: Transport,
        base: Url,
        catalogue: Catalogue,
        team_key: Option<String>,
    ) -> Self {
        let paths = Paths::new(base, catalogue.slug().clone());
        Self {
            transport,
            paths,
            catalogue,
            team_key,
            scopes: RefCell::new(None),
            refusals: Vec::new(),
            notice: None,
        }
    }

    /// # Errors
    ///
    /// [`ClientError`] for a missing or refused token, an unreadable config,
    /// or a missing or unreadable catalogue.
    pub fn from_config(
        context: &CredentialContext<'_>,
        integrations_root: &Path,
        transport_config: TransportConfig,
    ) -> Result<Self, ClientError> {
        let base = Url::parse(PRODUCTION_BASE).map_err(|error| {
            ClientError::TlsUnavailable {
                detail: error.to_string(),
            }
        })?;
        Self::from_config_at(base, context, integrations_root, transport_config)
    }

    /// As [`Self::from_config`], against the API base `base`.
    ///
    /// # Errors
    ///
    /// As [`Self::from_config`].
    pub fn from_config_at(
        base: Url,
        context: &CredentialContext<'_>,
        integrations_root: &Path,
        transport_config: TransportConfig,
    ) -> Result<Self, ClientError> {
        let credentials = resolve_credentials(context)?;
        let team_key = crate::auth::team_key(context.provenance.config)?;
        let catalogue = load_catalogue(integrations_root)?;
        let transport = Transport::new(
            &credentials,
            transport_config,
            Box::new(SystemSleeper),
            Box::new(ClockJitter),
        )?;
        let mut client = Self::new(transport, base, catalogue, team_key);
        client.refusals = credentials.refusals;
        client.notice = credentials.notice;
        Ok(client)
    }

    #[must_use]
    pub fn refusals(&self) -> &[Refusal] {
        &self.refusals
    }

    #[must_use]
    pub const fn notice(&self) -> Option<&Notice> {
        self.notice.as_ref()
    }

    #[must_use]
    pub const fn catalogue(&self) -> &Catalogue {
        &self.catalogue
    }

    #[must_use]
    pub const fn paths(&self) -> &Paths {
        &self.paths
    }

    #[must_use]
    pub const fn transport(&self) -> &Transport {
        &self.transport
    }

    /// The base team, as `shortcut.team_key` names it.
    ///
    /// # Errors
    ///
    /// [`TrackerError::Unconfigured`] when the key is unset or names no team.
    pub fn base_team(&self) -> Result<&Team, TrackerError> {
        let key = self.team_key.as_deref().ok_or_else(no_team_key)?;
        self.catalogue.team_for_key(key).map_err(|error| {
            TrackerError::Unconfigured {
                detail: error.to_string(),
            }
        })
    }

    /// Sends one write and classifies its failure for `operation`.
    ///
    /// # Errors
    ///
    /// The [`TrackerError`] `classify` gives the failure.
    pub fn write(
        &self,
        operation: Operation,
        method: &Method,
        url: &Url,
        body: &Value,
        resend: Resend,
    ) -> Result<Value, TrackerError> {
        let delivery = self.transport.send(
            method,
            url,
            body,
            resend,
            &self.transport.deadline(),
        );
        self.answered(operation, delivery)
    }

    /// Sends one read and returns the JSON of a 2xx answer.
    ///
    /// # Errors
    ///
    /// [`ReadFailure`], which keeps a non-2xx status for the caller.
    pub fn read(&self, url: &Url) -> Result<Value, ReadFailure> {
        let delivery = self.transport.get(url, &self.transport.deadline());
        match delivery.result {
            Err(error) => Err(ReadFailure::Unsent(error.to_string())),
            Ok(received) if !received.succeeded() => Err(ReadFailure::Status {
                status: received.status,
                detail: self.explained(&received),
            }),
            Ok(received) => received.json().ok_or_else(|| {
                ReadFailure::Unsent("Shortcut answered non-JSON".to_owned())
            }),
        }
    }

    fn answered(
        &self,
        operation: Operation,
        delivery: Delivery,
    ) -> Result<Value, TrackerError> {
        let may_have_applied = delivery.may_have_applied;
        let failed = |outcome, detail: &str| {
            classify(
                Failure {
                    outcome,
                    may_have_applied,
                },
                operation,
                detail,
            )
        };
        match delivery.result {
            Err(error) => Err(failed(outcome_of(&error), &error.to_string())),
            Ok(received) if !received.succeeded() => Err(failed(
                Outcome::Status(received.status),
                &self.explained(&received),
            )),
            Ok(received) => received.json().ok_or_else(|| {
                failed(Outcome::NonJsonBody, "Shortcut answered non-JSON")
            }),
        }
    }

    fn explained(&self, received: &Received) -> String {
        if received.status != 403 {
            return explain(received, &[]);
        }
        explain(received, &self.scopes())
    }

    fn scopes(&self) -> Vec<String> {
        if let Some(scopes) = self.scopes.borrow().as_ref() {
            return scopes.clone();
        }
        let scopes = whoami(
            &self.transport,
            self.paths.base(),
            &self.transport.deadline(),
        )
        .map(|me| me.scopes)
        .unwrap_or_default();
        *self.scopes.borrow_mut() = Some(scopes.clone());
        scopes
    }

    fn create_state(&self, team: &Team) -> Result<i64, TrackerError> {
        let workflow = team
            .default_workflow_id
            .and_then(|id| self.catalogue.workflow(id));
        let first_unstarted = workflow.and_then(|workflow| {
            workflow
                .states
                .iter()
                .filter(|state| state.kind == StateType::Unstarted)
                .min_by_key(|state| state.position)
                .map(|state| state.id)
        });
        if let Some(state) = first_unstarted {
            return Ok(state);
        }
        let fallback = whoami(
            &self.transport,
            self.paths.base(),
            &self.transport.deadline(),
        )
        .ok()
        .and_then(|me| me.default_state_id)
        .filter(|state| {
            workflow.is_some_and(|workflow| {
                workflow.states.iter().any(|known| known.id == *state)
            })
        });
        fallback.ok_or_else(|| TrackerError::Unconfigured {
            detail: format!(
                "the Shortcut team {:?} has no unstarted state in its default \
                 workflow to create a story in; run `accelerator shortcut init \
                 discover`",
                team.name
            ),
        })
    }

    fn show_issue(
        &self,
        id: &ExternalId,
        key: ShortcutKey,
    ) -> Result<RemoteIssue, ReadFailure> {
        let url =
            with_query(self.paths.entity(key), &[("fields", SHOW_FIELDS)]);
        let delivery = self.transport.get(&url, &self.transport.deadline());
        let received = match delivery.result {
            Err(error) => return Err(ReadFailure::Unsent(error.to_string())),
            Ok(received) if !received.succeeded() => {
                return Err(ReadFailure::Status {
                    status: received.status,
                    detail: self.explained(&received),
                });
            }
            Ok(received) => received,
        };
        let project = |op| {
            remote_projection::project_raw(
                remote_projection::Integration::Shortcut,
                op,
                &received.body,
            )
            .map_err(|error| ReadFailure::Unsent(error.to_string()))
        };
        let body = project(remote_projection::Op::Body)?;
        let stamp = project(remote_projection::Op::Updated)?;
        Ok(RemoteIssue {
            key: id.clone(),
            updated: timestamp(&stamp),
            body: tracker_support::port_body(&body),
        })
    }

    fn fetch_stories(
        &self,
        requested: &[(&ExternalId, ShortcutKey)],
        deadline: &Deadline,
        outcome: &mut FetchOutcome,
    ) {
        let cap = pages(self.transport.config().keyed_read_max_pages);
        for chunk in requested.chunks(KEYED_CHUNK) {
            let numbers: BTreeMap<u64, Vec<&ExternalId>> =
                chunk
                    .iter()
                    .fold(BTreeMap::new(), |mut numbers, (id, key)| {
                        numbers
                            .entry(key.number())
                            .or_insert_with(Vec::new)
                            .push(*id);
                        numbers
                    });
            let ids = numbers
                .keys()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let url = with_query(
                self.paths.collection(Collection::Stories),
                &[
                    ("ids", &ids),
                    ("filter", "include_archived"),
                    ("limit", PAGE_LIMIT),
                    ("fields", STAMP_FIELDS),
                ],
            );
            let read =
                collect(&self.transport, &self.paths, &url, cap, deadline);
            let (stamps, completeness) = match read {
                Ok(pages) => {
                    let stamps = stamps_by_number(&pages.entities);
                    let unrequested = stamps
                        .keys()
                        .any(|number| !numbers.contains_key(number));
                    let completeness = if unrequested {
                        Completeness::Transient
                    } else {
                        completeness_of(pages.cut)
                    };
                    (stamps, completeness)
                }
                Err(_) => (BTreeMap::new(), Completeness::Transient),
            };
            outcome.completeness = outcome.completeness.merge(completeness);
            for (number, ids) in numbers {
                for id in ids {
                    match stamps.get(&number) {
                        Some(stamp) => {
                            outcome.found.push((id.clone(), stamp.clone()));
                        }
                        None if completeness.is_complete() => {
                            outcome.absent.push(id.clone());
                        }
                        None => outcome.indeterminate.push(id.clone()),
                    }
                }
            }
        }
    }

    fn fetch_epics(
        &self,
        requested: &[(&ExternalId, ShortcutKey)],
        deadline: &Deadline,
        outcome: &mut FetchOutcome,
    ) {
        for (id, key) in requested {
            if deadline.expired() {
                outcome.indeterminate.push((*id).clone());
                outcome.completeness =
                    outcome.completeness.merge(Completeness::Transient);
                continue;
            }
            let url = with_query(
                self.paths.entity(*key),
                &[("fields", STAMP_FIELDS)],
            );
            let delivery = self.transport.get(&url, deadline);
            match delivery.result {
                Ok(received) if received.succeeded() => {
                    let stamp = received
                        .json()
                        .and_then(|body| {
                            body.pointer("/entity/updated_at")
                                .and_then(Value::as_str)
                                .map(str::to_owned)
                        })
                        .unwrap_or_default();
                    outcome.found.push(((*id).clone(), timestamp(&stamp)));
                }
                Ok(received) if received.status == 404 => {
                    outcome.absent.push((*id).clone());
                }
                _ => {
                    outcome.indeterminate.push((*id).clone());
                    outcome.completeness =
                        outcome.completeness.merge(Completeness::Transient);
                }
            }
        }
    }

    fn search_team(
        &self,
        uuid: &str,
        filters: &SearchFilters,
        whole_workspace: bool,
        found: &mut BTreeMap<u64, RemoteTimestamp>,
    ) -> Result<Completeness, TrackerError> {
        let name = match self.read(&self.paths.team(uuid)) {
            Ok(team) => team
                .pointer("/entity/name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            Err(ReadFailure::Status { status: 404, .. }) => {
                return Err(TrackerError::Unconfigured {
                    detail: format!(
                        "the Shortcut team {uuid} no longer exists; run \
                         `accelerator shortcut init discover` and update \
                         shortcut.team_key or shortcut.pull.additional_teams"
                    ),
                });
            }
            Err(failure) => {
                return Err(TrackerError::Retryable {
                    detail: format!("shortcut read: {failure}"),
                });
            }
        };
        let queries = match filters.queries(&name) {
            Ok(queries) => queries,
            Err(refusal) if whole_workspace => {
                tracing::warn!(team = %name, "skipping a Shortcut team: {refusal}");
                return Ok(Completeness::Transient);
            }
            Err(refusal) => {
                return Err(TrackerError::Unconfigured {
                    detail: refusal.to_string(),
                });
            }
        };
        let cap = pages(self.transport.config().discovery_max_pages);
        let deadline = self.transport.deadline();
        let mut completeness = Completeness::Complete;
        for query in queries {
            let url = with_query(
                self.paths.search_stories(),
                &[("query", &query), ("limit", PAGE_LIMIT)],
            );
            let result =
                collect(&self.transport, &self.paths, &url, cap, &deadline)
                    .map_err(|failure| TrackerError::Retryable {
                        detail: format!(
                            "shortcut search: {}",
                            page_detail(&failure)
                        ),
                    })?;
            completeness = completeness.merge(completeness_of(result.cut));
            for story in &result.entities {
                let in_team = story.pointer("/team/id").and_then(Value::as_str)
                    == Some(uuid);
                let Some(number) = story.get("id").and_then(Value::as_u64)
                else {
                    continue;
                };
                if !in_team {
                    tracing::debug!(
                        number,
                        "dropping a search result from another team"
                    );
                    continue;
                }
                let stamp = story
                    .get("updated_at")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                found.insert(number, timestamp(stamp));
            }
        }
        Ok(completeness)
    }

    fn validate_filters(
        &self,
        filters: &SearchFilters,
        scope: &EntityScope,
    ) -> Result<(), ScopeError> {
        let states = filters.states();
        if states.is_empty() {
            return Ok(());
        }
        let workflow_ids: Vec<i64> = match scope {
            EntityScope::WholeWorkspace => self
                .catalogue
                .workflows()
                .iter()
                .map(|workflow| workflow.id)
                .collect(),
            EntityScope::Keyed { base, additional } => {
                let mut ids = Vec::new();
                for key in base.iter().chain(additional) {
                    let team = self.catalogue.team_for_key(key).map_err(|error| match error {
                        TeamLookupError::Unknown { .. } => ScopeError {
                            detail: format!(
                                "the Shortcut team {key:?} is not in catalogue.json, so a \
                                 state filter cannot be checked against its workflows; \
                                 run `accelerator shortcut init discover`"
                            ),
                        },
                        colliding @ TeamLookupError::Colliding { .. } => ScopeError {
                            detail: colliding.to_string(),
                        },
                    })?;
                    ids.extend(team.workflow_ids.iter().copied());
                }
                ids
            }
        };
        let known: Vec<&str> = self
            .catalogue
            .workflows()
            .iter()
            .filter(|workflow| workflow_ids.contains(&workflow.id))
            .flat_map(|workflow| {
                workflow.states.iter().map(|state| state.name.as_str())
            })
            .collect();
        states.iter().find(|state| !known.contains(state)).map_or(
            Ok(()),
            |unknown| {
                Err(ScopeError {
                    detail: format!(
                        "the Shortcut state filter {unknown:?} names no state of \
                         the teams in scope. Valid states: {}. A state added \
                         since the last discovery appears after `accelerator \
                         shortcut init discover`",
                        known.join(", ")
                    ),
                })
            },
        )
    }
}

/// Why a read did not yield a 2xx JSON answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadFailure {
    Status { status: u16, detail: String },
    Unsent(String),
}

impl std::fmt::Display for ReadFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (Self::Status { detail, .. } | Self::Unsent(detail)) = self;
        formatter.write_str(detail)
    }
}

impl std::error::Error for ReadFailure {}

impl BodyDialect for ShortcutClient {
    fn is_wire_document(&self, _body: &str) -> bool {
        false
    }
}

impl RemoteTracker for ShortcutClient {
    fn create(
        &self,
        title: &str,
        body: &str,
        kind: &str,
    ) -> Result<ExternalId, TrackerError> {
        let entity = NewEntity::for_kind(kind).map_err(|unknown| {
            TrackerError::Rejected {
                detail: format!("shortcut create: {unknown}"),
            }
        })?;
        let (collection, request) = match entity {
            NewEntity::Epic => (
                Collection::Epics,
                json!({"name": title, "description": body}),
            ),
            NewEntity::Story(story_type) => {
                let team = self.base_team()?;
                let state = self.create_state(team)?;
                (
                    Collection::Stories,
                    json!({
                        "name": title,
                        "description": body,
                        "story_type": story_type.as_str(),
                        "team_id": team.id,
                        "workflow_state_id": state,
                    }),
                )
            }
        };
        let created = self.write(
            Operation::Create,
            &Method::POST,
            &self.paths.collection(collection),
            &request,
            Resend::OnlyWhenThrottled,
        )?;
        let unusable = || {
            classify(
                Failure {
                    outcome: Outcome::UnusableBody,
                    may_have_applied: false,
                },
                Operation::Create,
                "Shortcut created the entity but its answer held no usable id",
            )
        };
        let number = created
            .pointer("/entity/id")
            .and_then(Value::as_u64)
            .filter(|number| *number > 0)
            .ok_or_else(unusable)?;
        let key = match collection {
            Collection::Stories => format!("sc-{number}"),
            Collection::Epics => format!("epic-{number}"),
        };
        tracker_support::issue_key_is_safe(&key).map_err(|_| unusable())?;
        Ok(ExternalId::new(key))
    }

    fn update(
        &self,
        id: &ExternalId,
        title: Option<&str>,
        body: &str,
    ) -> Result<(), TrackerError> {
        let key = parsed_for_write(id, Operation::Update)?;
        let fields = UpdateFields {
            name: title.map(str::to_owned),
            description: Some(body.to_owned()),
            ..UpdateFields::default()
        };
        self.write(
            Operation::Update,
            &Method::PATCH,
            &self.paths.entity(key),
            &fields.body(),
            Resend::WhileRetryable,
        )
        .map(drop)
    }

    fn retitle(
        &self,
        id: &ExternalId,
        title: &str,
    ) -> Result<(), TrackerError> {
        let key = parsed_for_write(id, Operation::Update)?;
        let fields = UpdateFields {
            name: Some(title.to_owned()),
            ..UpdateFields::default()
        };
        self.write(
            Operation::Update,
            &Method::PATCH,
            &self.paths.entity(key),
            &fields.body(),
            Resend::WhileRetryable,
        )
        .map(drop)
    }

    fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError> {
        let key = parsed_for_read(id)?;
        self.show_issue(id, key)
            .map_err(|failure| TrackerError::Retryable {
                detail: format!("shortcut read {id}: {failure}"),
            })
    }

    fn locate(&self, id: &ExternalId) -> Result<Located, TrackerError> {
        let key = parsed_for_read(id)?;
        match self.show_issue(id, key) {
            Ok(issue) => Ok(Located::Found(issue)),
            Err(ReadFailure::Status { status: 404, .. }) => {
                Ok(Located::NotFound)
            }
            Err(failure) => Err(TrackerError::Retryable {
                detail: format!("shortcut read {id}: {failure}"),
            }),
        }
    }

    fn fetch_all(
        &self,
        ids: &[ExternalId],
    ) -> Result<FetchOutcome, TrackerError> {
        let mut outcome = FetchOutcome {
            found: Vec::new(),
            absent: Vec::new(),
            indeterminate: Vec::new(),
            completeness: Completeness::Complete,
        };
        let mut stories = Vec::new();
        let mut epics = Vec::new();
        for id in ids {
            match ShortcutKey::parse(id.as_str()) {
                Ok(key @ ShortcutKey::Story(_)) => stories.push((id, key)),
                Ok(key @ ShortcutKey::Epic(_)) => epics.push((id, key)),
                Err(_) => outcome.indeterminate.push(id.clone()),
            }
        }
        let deadline = self.transport.deadline();
        self.fetch_stories(&stories, &deadline, &mut outcome);
        self.fetch_epics(&epics, &deadline, &mut outcome);
        Ok(outcome)
    }

    fn search(&self, scope: &SearchScope) -> Result<Discovery, TrackerError> {
        let filters =
            SearchFilters::parse(&scope.filters).map_err(|refusal| {
                TrackerError::Unconfigured {
                    detail: refusal.to_string(),
                }
            })?;
        let (uuids, whole_workspace) = match &scope.entities {
            EntityScope::Keyed { base, additional } => {
                let base_only = additional.is_empty();
                (
                    base.iter().chain(additional).cloned().collect::<Vec<_>>(),
                    !base_only && base.is_none(),
                )
            }
            EntityScope::WholeWorkspace => {
                return Err(TrackerError::Unconfigured {
                    detail: "a whole-workspace Shortcut search reaches the client only \
                             after the engine enumerates its teams"
                        .to_owned(),
                });
            }
        };
        let mut found = BTreeMap::new();
        let mut completeness = Completeness::Complete;
        for uuid in uuids {
            completeness = completeness.merge(self.search_team(
                &uuid,
                &filters,
                whole_workspace,
                &mut found,
            )?);
        }
        Ok(Discovery {
            found: found
                .into_iter()
                .map(|(number, stamp)| {
                    (ExternalId::new(format!("sc-{number}")), stamp)
                })
                .collect(),
            completeness,
        })
    }

    fn resolve_scope(
        &self,
        scope: &SearchScope,
    ) -> Result<SearchScope, ScopeError> {
        let filters =
            SearchFilters::parse(&scope.filters).map_err(|refusal| {
                ScopeError {
                    detail: refusal.to_string(),
                }
            })?;
        let refuse = |error: TeamLookupError| ScopeError {
            detail: error.to_string(),
        };
        if let EntityScope::Keyed { base, additional } = &scope.entities {
            for key in base.iter().chain(additional) {
                if let Err(colliding @ TeamLookupError::Colliding { .. }) =
                    self.catalogue.team_for_key(key)
                {
                    return Err(refuse(colliding));
                }
            }
        }
        self.validate_filters(&filters, &scope.entities)?;
        let entities = match &scope.entities {
            EntityScope::Keyed { base, additional }
                if additional.is_empty() =>
            {
                let key = base.as_deref().ok_or_else(|| ScopeError {
                    detail: no_team_key().into_detail(),
                })?;
                let team = self.catalogue.team_for_key(key).map_err(refuse)?;
                EntityScope::Keyed {
                    base: Some(team.id.clone()),
                    additional: Vec::new(),
                }
            }
            broadened => broadened.clone(),
        };
        Ok(SearchScope {
            entities,
            filters: scope.filters.clone(),
        })
    }

    fn enumerate_visible_entities(
        &self,
    ) -> Result<Vec<VisibleEntity>, TrackerError> {
        let deadline = self.transport.deadline();
        let teams = collect(
            &self.transport,
            &self.paths,
            &self.paths.teams(),
            None,
            &deadline,
        )
        .map_err(|failure| TrackerError::Retryable {
            detail: format!("shortcut teams: {}", page_detail(&failure)),
        })?;
        if let Some(cut) = teams.cut {
            return Err(TrackerError::Retryable {
                detail: format!(
                    "shortcut teams: the listing stopped early ({})",
                    cut.cause()
                ),
            });
        }
        let identities: Vec<TeamIdentity> = teams
            .entities
            .iter()
            .filter_map(|team| {
                Some(TeamIdentity {
                    uuid: team.get("id")?.as_str()?.to_owned(),
                    mention_name: team
                        .get("mention_name")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    name: team
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                })
            })
            .collect();
        let keys = TeamKeys::over(&identities);
        Ok(identities
            .iter()
            .filter_map(|team| {
                Some(VisibleEntity {
                    key: keys.visible_key(&team.uuid)?,
                    identifier: team.uuid.clone(),
                    name: team.name.clone(),
                })
            })
            .collect())
    }

    fn preview_create(
        &self,
        kind: &str,
    ) -> Result<CreatePreview, TrackerError> {
        let issue_type = match NewEntity::for_kind(kind) {
            Ok(_) if kind.is_empty() => FieldResolution::Unset,
            Ok(entity) => {
                FieldResolution::Resolved(entity.type_name().to_owned())
            }
            Err(_) => FieldResolution::Unresolvable(kind.to_owned()),
        };
        let project =
            if matches!(NewEntity::for_kind(kind), Ok(NewEntity::Epic)) {
                FieldResolution::Unset
            } else {
                self.team_key
                    .as_deref()
                    .map_or(FieldResolution::Unset, |key| {
                        self.catalogue.team_for_key(key).map_or_else(
                            |_| FieldResolution::Unresolvable(key.to_owned()),
                            |team| FieldResolution::Resolved(team.name.clone()),
                        )
                    })
            };
        Ok(CreatePreview {
            project,
            issue_type,
        })
    }

    fn validate_update(
        &self,
        _id: &ExternalId,
        title: Option<&str>,
        _body: Option<&str>,
    ) -> ValidationOutcome {
        if title.is_some_and(|title| title.trim().is_empty()) {
            ValidationOutcome::Rejected {
                reasons: vec![
                    "a Shortcut story or epic needs a non-empty title"
                        .to_owned(),
                ],
            }
        } else {
            ValidationOutcome::Valid
        }
    }
}

fn load_catalogue(integrations_root: &Path) -> Result<Catalogue, ClientError> {
    let fs = SystemFilesystem::new(integrations_root.to_path_buf());
    ShortcutCache::new(&fs, integrations_root.join("shortcut"))
        .load()
        .map_err(ClientError::Catalogue)?
        .ok_or(ClientError::NoCatalogue)
}

fn no_team_key() -> TrackerError {
    TrackerError::Unconfigured {
        detail: "shortcut.team_key is not set; run `accelerator shortcut init \
                 list-teams` to see the team keys, then set one"
            .to_owned(),
    }
}

fn parsed_for_read(id: &ExternalId) -> Result<ShortcutKey, TrackerError> {
    ShortcutKey::parse(id.as_str()).map_err(|unparseable| {
        TrackerError::Retryable {
            detail: format!("shortcut read: {unparseable}"),
        }
    })
}

fn parsed_for_write(
    id: &ExternalId,
    operation: Operation,
) -> Result<ShortcutKey, TrackerError> {
    ShortcutKey::parse(id.as_str()).map_err(|unparseable| {
        classify(
            Failure {
                outcome: Outcome::RequestInvalid,
                may_have_applied: false,
            },
            operation,
            &unparseable.to_string(),
        )
    })
}

const fn outcome_of(error: &ClientError) -> Outcome {
    match error {
        ClientError::NotSent { .. } => Outcome::NotSent,
        ClientError::RequestInvalid { .. }
        | ClientError::ForeignPage { .. } => Outcome::RequestInvalid,
        _ => Outcome::Transport,
    }
}

fn timestamp(stamp: &str) -> RemoteTimestamp {
    if stamp.is_empty() {
        RemoteTimestamp::NotReported
    } else {
        RemoteTimestamp::Reported(stamp.to_owned())
    }
}

fn stamps_by_number(entities: &[Value]) -> BTreeMap<u64, RemoteTimestamp> {
    entities
        .iter()
        .filter_map(|story| {
            let number = story.get("id")?.as_u64()?;
            let stamp = story
                .get("updated_at")
                .and_then(Value::as_str)
                .unwrap_or_default();
            Some((number, timestamp(stamp)))
        })
        .collect()
}

const fn completeness_of(cut: Option<Cut>) -> Completeness {
    match cut {
        None => Completeness::Complete,
        Some(Cut::PageCap) => Completeness::CapHit,
        Some(Cut::Deadline) => Completeness::Transient,
    }
}

const fn pages(ceiling: Ceiling) -> Option<usize> {
    match ceiling {
        Ceiling::Bounded(cap) => Some(cap),
        Ceiling::Unlimited => None,
    }
}

fn page_detail(failure: &PageError) -> String {
    match failure {
        PageError::Refused(received) => explain(received, &[]),
        PageError::Unusable(what) => what.clone(),
        PageError::Client(error) => error.to_string(),
        PageError::Repeated(url) => {
            format!("the same next page came back twice ({url})")
        }
    }
}

/// `url` with `pairs` appended to its query.
#[must_use]
pub fn with_query(mut url: Url, pairs: &[(&str, &str)]) -> Url {
    {
        let mut query = url.query_pairs_mut();
        for (name, value) in pairs {
            query.append_pair(name, value);
        }
    }
    url
}
