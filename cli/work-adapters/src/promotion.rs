//! Promoting a draft onto the issue a tracker creates for it.
//!
//! The service creates the remote issue, carries the tracker key into the
//! issue's H1, retires the draft ID to the key, and records the sync
//! baseline, recording each stage before the next, so any run can finish a
//! promotion another left.

use std::path::Path;
use std::path::PathBuf;

use corpus::StoreError;
use tracker::ExternalId;
use tracker::Located;
use tracker::RemoteIssue;
use tracker::RemoteTracker;
use work::draft_id::DraftId;
use work::identity::ItemIdentity;
use work::promotion::intended_baseline;
use work::promotion::next_step;
use work::promotion::retitling;
use work::promotion::without_ids;
use work::promotion::NotPromoted;
use work::promotion::Promotion;
use work::promotion::PromotionMode;
use work::promotion::PromotionRecord;
use work::promotion::PromotionStage;
use work::promotion::PromotionStep;
use work::promotion::ReadBack;
use work::promotion::RecordState;
use work::promotion::RemoteKeptReason;
use work::promotion::Retitling;
use work::retirement::retired_content;
use work::retirement::Retirement;
use work::retirement::RetirementCause;
use work::retirement::RetirementCauseKind;
use work::retirement::RetirementFailure;
use work::retirement::RetirementRefusal;
use work::sync::RequestFingerprint;

use crate::create_request_fields;
use crate::promotion_records::PromotionRecords;
use crate::promotion_records::StoredRecord;
use crate::remote_create::send_create;
use crate::remote_create::CreateRequest;
use crate::remote_create::RemoteCreate;
use crate::retirement::acquire_retirement_lock;
use crate::retirement::corpus_identities;
use crate::retirement::finish_retirement;
use crate::retirement::plan_from_corpus;
use crate::retirement::retirement_outstanding;
use crate::retirement::FinishFailure;
use crate::retirement::RetirementLockGuard;
use crate::retirement::RetirementPorts;
use crate::sync::created_baseline::record_created_baseline;
use crate::sync::digest;
use crate::sync::pending_push;

/// What became of one draft a run set out to promote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromotionOutcome {
    Previewed,
    Promoted(Promotion),
    /// `held_key` names the issue the draft's record holds, once one
    /// exists.
    NotPromoted {
        reason: NotPromoted,
        held_key: Option<ExternalId>,
    },
}

/// Where a person looks to act on a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailSource {
    /// The item already claiming the key, or the file in the target's way.
    Holder,
    /// A path to restore from version control.
    Vcs,
    /// A path to restore from its copy in the recovery directory at
    /// `location`.
    Recovery { location: PathBuf },
    /// The row's own item: the draft, or the promoted item a conflict will
    /// be raised for.
    Item,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detail {
    pub source: DetailSource,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionRow {
    pub draft: String,
    /// Where the draft was when the run began.
    pub path: PathBuf,
    pub outcome: PromotionOutcome,
    /// What a person needs to look at: every unpromoted draft names at
    /// least one path, and so does a promotion the next sync will find in
    /// conflict.
    pub details: Vec<Detail>,
}

impl PromotionRow {
    #[must_use]
    pub fn previewed(draft: &DraftId, path: PathBuf) -> Self {
        Self {
            draft: draft.as_str().to_owned(),
            path,
            outcome: PromotionOutcome::Previewed,
            details: Vec::new(),
        }
    }

    /// The row for a promotion that ran over `ports`, naming the key its
    /// record holds when it stopped short, and the paths a person needs.
    /// `state_dir` is where recovery directories live.
    #[must_use]
    pub fn of(
        draft: &DraftId,
        path: PathBuf,
        result: Result<Promotion, NotPromoted>,
        ports: &PromotionPorts<'_>,
        state_dir: &Path,
    ) -> Self {
        let (outcome, details) = match result {
            Ok(promotion) => (
                PromotionOutcome::Promoted(promotion.clone()),
                conflict_subject(&promotion, ports.retirement),
            ),
            Err(reason) => {
                let held_key = held_key(&reason, draft, ports.records);
                let details = unpromoted_subjects(
                    &reason,
                    draft,
                    held_key.as_ref(),
                    &path,
                    ports.retirement,
                    state_dir,
                );
                (PromotionOutcome::NotPromoted { reason, held_key }, details)
            }
        };
        Self {
            draft: draft.as_str().to_owned(),
            path,
            outcome,
            details,
        }
    }
}

fn conflict_subject(
    promotion: &Promotion,
    ports: &RetirementPorts<'_>,
) -> Vec<Detail> {
    let Promotion::Completed(key, work::sync::SyncState::Conflict) = promotion
    else {
        return Vec::new();
    };
    corpus_identities(ports)
        .ok()
        .and_then(|items| {
            items.into_iter().find(|item| same(&item.id, key.as_str()))
        })
        .map(|item| Detail {
            source: DetailSource::Item,
            path: item.path,
        })
        .into_iter()
        .collect()
}

fn unpromoted_subjects(
    reason: &NotPromoted,
    draft: &DraftId,
    held_key: Option<&ExternalId>,
    draft_path: &Path,
    ports: &RetirementPorts<'_>,
    state_dir: &Path,
) -> Vec<Detail> {
    let detail = |source, path: &Path| Detail {
        source,
        path: path.to_path_buf(),
    };
    match reason {
        NotPromoted::Refused(
            RetirementRefusal::IdTaken { holder, .. }
            | RetirementRefusal::KeyLinked { holder }
            | RetirementRefusal::TargetExists(holder),
        ) => vec![detail(DetailSource::Holder, holder)],
        NotPromoted::RetirementFailed(
            RetirementFailure::RestoreIncomplete { unrestored, .. },
        ) => {
            let recovery_dir = held_key.map(|key| {
                Retirement {
                    old_id: draft.as_str(),
                    new_id: key.as_str(),
                    new_external_id: Some(key.as_str()),
                }
                .recovery_dir()
            });
            unrestored
                .iter()
                .map(|path| {
                    let copied = recovery_dir.as_ref().filter(|dir| {
                        !ports.files.recovery.copy_settled(dir, path)
                    });
                    detail(
                        copied.map_or(DetailSource::Vcs, |dir| {
                            DetailSource::Recovery {
                                location: state_dir.join(dir),
                            }
                        }),
                        path,
                    )
                })
                .collect()
        }
        _ => vec![detail(DetailSource::Item, draft_path)],
    }
}

/// The issue a draft left unpromoted already has: named by the reason, or
/// held by the draft's record.
#[must_use]
pub fn held_key(
    reason: &NotPromoted,
    draft: &DraftId,
    records: &dyn PromotionRecords,
) -> Option<ExternalId> {
    let named = match reason {
        NotPromoted::ReadBackFailed(key)
        | NotPromoted::RecordUnwritable { key: Some(key), .. }
        | NotPromoted::AdoptConflictsWithRecordedKey { recorded: key } => {
            Some(key.clone())
        }
        _ => None,
    };
    named.or_else(|| records.read(draft).key())
}

pub struct PromotionPorts<'a> {
    pub tracker: &'a dyn RemoteTracker,
    pub retirement: &'a RetirementPorts<'a>,
    pub records: &'a dyn PromotionRecords,
}

/// The draft as the tracker is asked to create it.
struct DraftContent {
    path: PathBuf,
    content: String,
    title: String,
    body: String,
    kind: String,
}

impl DraftContent {
    fn projected(&self) -> String {
        format!("{}\n{}", self.title, self.body)
    }
}

fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

const fn store_failure(path: PathBuf, error: StoreError) -> NotPromoted {
    NotPromoted::RetirementFailed(RetirementFailure::RolledBack {
        cause: RetirementCause {
            path,
            kind: RetirementCauseKind::Store(error),
        },
    })
}

fn unwritable(key: Option<&ExternalId>, error: &StoreError) -> NotPromoted {
    NotPromoted::RecordUnwritable {
        key: key.cloned(),
        detail: error.to_string(),
    }
}

fn from_finish(failure: FinishFailure) -> NotPromoted {
    match failure {
        FinishFailure::Refused(refusal) => NotPromoted::Refused(refusal),
        FinishFailure::Failed(failure) => {
            NotPromoted::RetirementFailed(failure)
        }
    }
}

fn identities(
    ports: &RetirementPorts<'_>,
) -> Result<Vec<ItemIdentity>, NotPromoted> {
    corpus_identities(ports).map_err(from_finish)
}

const fn same(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn read_draft(
    item: &ItemIdentity,
    ports: &RetirementPorts<'_>,
) -> Result<DraftContent, NotPromoted> {
    let unreadable = |detail: String| {
        store_failure(
            item.path.clone(),
            StoreError::Io {
                path: item.path.display().to_string(),
                detail,
            },
        )
    };
    let content = ports
        .files
        .reader
        .read(&item.path)
        .map_err(|error| unreadable(error.to_string()))?
        .ok_or_else(|| unreadable("the draft has gone".to_owned()))?;
    let (_, body) = digest::split_frontmatter_and_body(&content)
        .map_err(|error| unreadable(error.to_string()))?;
    let fields = create_request_fields::read(&content)
        .map_err(|error| unreadable(error.to_string()))?;
    Ok(DraftContent {
        path: item.path.clone(),
        title: fields.title,
        kind: fields.kind,
        body,
        content,
    })
}

fn read_back_of(issue: &RemoteIssue) -> ReadBack {
    ReadBack {
        hash: digest::remote_body(&issue.body),
        updated: issue.updated.clone(),
    }
}

/// Whether two projected bodies agree once every ID is set aside.
fn agree_but_for_ids(left: &str, right: &str, ids: &[&str]) -> bool {
    digest::remote_body(&without_ids(left, ids))
        == digest::remote_body(&without_ids(right, ids))
}

const fn record_state(stored: &StoredRecord) -> RecordState<'_> {
    match stored {
        StoredRecord::Absent => RecordState::Absent,
        StoredRecord::Unreadable(_) => RecordState::Unreadable,
        StoredRecord::Present(record) => RecordState::Present(record),
    }
}

/// Promotes `draft`, resuming from wherever its record says an earlier run
/// stopped, under the retirement lock throughout.
///
/// # Errors
///
/// A [`NotPromoted`] naming why the draft is not yet promoted; the record
/// is left at the stage the next promotion resumes from.
pub fn promote(
    draft: &DraftId,
    mode: &PromotionMode,
    ports: &PromotionPorts<'_>,
) -> Result<Promotion, NotPromoted> {
    let lock =
        acquire_retirement_lock(ports.retirement.lock).map_err(|error| {
            store_failure(ports.retirement.layout.work_dir.to_path_buf(), error)
        })?;
    let items = identities(ports.retirement)?;
    let Some(item) = items.iter().find(|item| same(&item.id, draft.as_str()))
    else {
        return already_promoted(draft, &items, ports, &lock);
    };
    let content = read_draft(item, ports.retirement)?;
    let promotion = Promoting {
        draft,
        content: &content,
        ports,
        lock: &lock,
    };
    loop {
        let stored = ports.records.read(draft);
        match next_step(record_state(&stored), mode) {
            PromotionStep::CreateIssue => promotion.create_issue()?,
            PromotionStep::VerifyThenAdopt(key) => {
                promotion.adopt(&key, &stored)?;
            }
            PromotionStep::RetitleRemote(key) => {
                promotion.retitle_remote(&key, &stored)?;
            }
            PromotionStep::Retire(key) => {
                return promotion.retire(&key, &stored);
            }
            PromotionStep::Finish => {
                let StoredRecord::Present(record) = stored else {
                    return Err(NotPromoted::CreateOutcomeUnknown);
                };
                return finish_promotion(
                    &record,
                    ports.retirement,
                    ports.records,
                    &lock,
                );
            }
            PromotionStep::Stop(reason) => return Err(reason),
        }
    }
}

/// A draft ID no item holds as its `id`: promoted already when an item
/// carries it as an alias, whose outstanding record is finished first.
fn already_promoted(
    draft: &DraftId,
    items: &[ItemIdentity],
    ports: &PromotionPorts<'_>,
    lock: &RetirementLockGuard,
) -> Result<Promotion, NotPromoted> {
    let Some(holder) = items.iter().find(|item| {
        item.aliases.iter().any(|alias| same(alias, draft.as_str()))
    }) else {
        return Err(NotPromoted::Refused(RetirementRefusal::ItemNotFound(
            draft.as_str().to_owned(),
        )));
    };
    if let StoredRecord::Present(record) = ports.records.read(draft) {
        if matches!(record.stage, PromotionStage::Retiring { .. }) {
            finish_promotion(&record, ports.retirement, ports.records, lock)?;
        }
    }
    Ok(Promotion::AlreadyDone(ExternalId::new(holder.id.clone())))
}

struct Promoting<'a> {
    draft: &'a DraftId,
    content: &'a DraftContent,
    ports: &'a PromotionPorts<'a>,
    lock: &'a RetirementLockGuard,
}

impl Promoting<'_> {
    fn record(
        &self,
        request: RequestFingerprint,
        stage: PromotionStage,
    ) -> PromotionRecord {
        let content = self.content;
        PromotionRecord {
            draft_id: self.draft.clone(),
            request,
            content_digest: pending_push::content_digest(
                &content.title,
                &content.body,
                &content.kind,
                self.draft.as_str(),
            ),
            stage,
        }
    }

    fn save(&self, record: &PromotionRecord) -> Result<(), NotPromoted> {
        self.ports
            .records
            .save(record)
            .map_err(|error| unwritable(record.stage.key(), &error))
    }

    fn advance(
        &self,
        stored: &StoredRecord,
        stage: PromotionStage,
    ) -> Result<(), NotPromoted> {
        let StoredRecord::Present(record) = stored else {
            return Err(NotPromoted::CreateOutcomeUnknown);
        };
        self.save(&PromotionRecord {
            stage,
            ..(**record).clone()
        })
    }

    fn fresh_request(&self) -> RequestFingerprint {
        let content = self.content;
        RequestFingerprint {
            title: content.title.clone(),
            digest: pending_push::request_digest(
                &content.title,
                &content.body,
                &content.kind,
            ),
            attempted_at: now_epoch(),
            failure: None,
        }
    }

    fn create_issue(&self) -> Result<(), NotPromoted> {
        let content = self.content;
        let request = self.fresh_request();
        self.save(&self.record(request.clone(), PromotionStage::Attempted))?;
        let created = send_create(
            &CreateRequest {
                title: &content.title,
                body: &content.body,
                kind: &content.kind,
            },
            self.ports.tracker,
        );
        let forget = || {
            let _ = self.ports.records.remove(self.draft);
        };
        match created {
            RemoteCreate::Created(key) => {
                let created_remote_hash = self
                    .ports
                    .tracker
                    .show(&key)
                    .ok()
                    .map(|issue| read_back_of(&issue).hash);
                self.save(&self.record(
                    request,
                    PromotionStage::Created {
                        key,
                        created_remote_hash,
                    },
                ))
            }
            RemoteCreate::TrackerUnreachable => {
                forget();
                Err(NotPromoted::TrackerUnreachable)
            }
            RemoteCreate::Rejected { detail } => {
                forget();
                Err(NotPromoted::RequestRejected { detail })
            }
            RemoteCreate::OutcomeUnknown { detail } => {
                self.save(&self.record(
                    RequestFingerprint {
                        failure: Some(detail),
                        ..request
                    },
                    PromotionStage::Attempted,
                ))?;
                Err(NotPromoted::CreateOutcomeUnknown)
            }
        }
    }

    /// Records the issue the user named as the draft's, once the tracker
    /// confirms it and nothing local already claims its key. The issue is
    /// never rewritten: its content is the user's.
    fn adopt(
        &self,
        named: &ExternalId,
        stored: &StoredRecord,
    ) -> Result<(), NotPromoted> {
        let issue = match self.ports.tracker.locate(named) {
            Ok(Located::Found(issue)) => issue,
            Ok(Located::NotFound) => {
                return Err(NotPromoted::AdoptedIssueMissing(named.clone()));
            }
            Err(_) => return Err(NotPromoted::TrackerUnreachable),
        };
        plan_from_corpus(&self.retirement(named), self.ports.retirement)
            .map_err(from_finish)?;
        let request = match stored {
            StoredRecord::Present(record) => record.request.clone(),
            StoredRecord::Absent | StoredRecord::Unreadable(_) => {
                self.fresh_request()
            }
        };
        self.save(&self.record(
            request,
            PromotionStage::RemoteKept {
                key: named.clone(),
                reason: RemoteKeptReason::UserNamedAdopt {
                    read_back: read_back_of(&issue),
                },
            },
        ))
    }

    fn retirement<'k>(&'k self, key: &'k ExternalId) -> Retirement<'k> {
        Retirement {
            old_id: self.draft.as_str(),
            new_id: key.as_str(),
            new_external_id: Some(key.as_str()),
        }
    }

    fn promoted_content(&self, key: &ExternalId) -> String {
        retired_content(
            &self.content.content,
            &self.retirement(key),
            &self.content.path,
            self.ports.retirement.layout.work_dir,
        )
    }

    fn show(&self, key: &ExternalId) -> Result<RemoteIssue, NotPromoted> {
        self.ports
            .tracker
            .show(key)
            .map_err(|_| NotPromoted::ReadBackFailed(key.clone()))
    }

    fn retitle_remote(
        &self,
        key: &ExternalId,
        stored: &StoredRecord,
    ) -> Result<(), NotPromoted> {
        let StoredRecord::Present(record) = stored else {
            return Err(NotPromoted::CreateOutcomeUnknown);
        };
        let PromotionStage::Created {
            created_remote_hash,
            ..
        } = &record.stage
        else {
            return Err(NotPromoted::CreateOutcomeUnknown);
        };
        if !corpus::is_tracker_key(key.as_str()) {
            tracing::warn!(
                "{key} is not a tracker key; prose references to it will \
                 not be rewritten if it is ever retired"
            );
        }
        let issue = self.show(key)?;
        let read_back = read_back_of(&issue);
        let ids = [self.draft.as_str(), key.as_str()];
        let matches_draft =
            agree_but_for_ids(&issue.body, &self.content.projected(), &ids);
        match retitling(
            created_remote_hash.as_deref(),
            &read_back,
            matches_draft,
        ) {
            Retitling::Keep(reason) => self.advance(
                stored,
                PromotionStage::RemoteKept {
                    key: key.clone(),
                    reason,
                },
            ),
            Retitling::Retitle => {
                let promoted = self.promoted_content(key);
                let (_, body) = digest::split_frontmatter_and_body(&promoted)
                    .unwrap_or_default();
                if self
                    .ports
                    .tracker
                    .update(key, &self.content.title, &body)
                    .is_err()
                {
                    return self.advance(
                        stored,
                        PromotionStage::RemoteKept {
                            key: key.clone(),
                            reason: RemoteKeptReason::UpdateFailed {
                                read_back,
                            },
                        },
                    );
                }
                let updated = self.show(key)?;
                self.advance(
                    stored,
                    PromotionStage::RemoteRetitled {
                        key: key.clone(),
                        read_back: read_back_of(&updated),
                    },
                )
            }
        }
    }

    fn retire(
        &self,
        key: &ExternalId,
        stored: &StoredRecord,
    ) -> Result<Promotion, NotPromoted> {
        let StoredRecord::Present(record) = stored else {
            return Err(NotPromoted::CreateOutcomeUnknown);
        };
        let issue = self.show(key)?;
        let retirement = self.retirement(key);
        let plan = plan_from_corpus(&retirement, self.ports.retirement)
            .map_err(from_finish)?;
        let promoted = plan
            .retired_item
            .unwrap_or_else(|| self.promoted_content(key));
        let local = |content: &str| digest::local(content).unwrap_or_default();
        let (_, promoted_body) =
            digest::split_frontmatter_and_body(&promoted).unwrap_or_default();
        let matches_promoted = agree_but_for_ids(
            &issue.body,
            &format!("{}\n{promoted_body}", self.content.title),
            &[self.draft.as_str(), key.as_str()],
        );
        let Some(baseline) = intended_baseline(
            &record.stage,
            &local(&self.content.content),
            &local(&promoted),
            matches_promoted,
        ) else {
            return Err(NotPromoted::CreateOutcomeUnknown);
        };
        let retiring = PromotionRecord {
            stage: PromotionStage::Retiring {
                key: key.clone(),
                baseline,
                recovery_dir: retirement.recovery_dir(),
                before: Box::new(record.stage.clone()),
            },
            ..(**record).clone()
        };
        self.save(&retiring)?;
        finish_promotion(
            &retiring,
            self.ports.retirement,
            self.ports.records,
            self.lock,
        )
    }
}

/// The one tail every promotion ends with, whoever runs it.
///
/// Retires the draft ID to the key if that is still outstanding, records
/// the intended baseline once the draft ID is only an alias of the key's
/// item, then removes the record. The completed promotion names the state
/// that baseline leaves the promoted item in.
///
/// A refused or rolled-back retirement rewinds the record to its stage
/// before `Retiring`, so no `Retiring` record stands for a retirement that
/// is not in progress.
///
/// # Errors
///
/// A [`NotPromoted`] when the retirement is refused or fails, or the
/// baseline or record cannot be written.
pub fn finish_promotion(
    record: &PromotionRecord,
    retirement_ports: &RetirementPorts<'_>,
    records: &dyn PromotionRecords,
    lock: &RetirementLockGuard,
) -> Result<Promotion, NotPromoted> {
    let PromotionStage::Retiring {
        key,
        baseline,
        before,
        ..
    } = &record.stage
    else {
        return Err(NotPromoted::CreateOutcomeUnknown);
    };
    let retirement = Retirement {
        old_id: record.draft_id.as_str(),
        new_id: key.as_str(),
        new_external_id: Some(key.as_str()),
    };
    let rewind = |failure: NotPromoted| {
        let _ = records.save(&PromotionRecord {
            stage: (**before).clone(),
            ..record.clone()
        });
        failure
    };
    let outstanding = retirement_outstanding(&retirement, retirement_ports)
        .map_err(|failure| rewind(from_finish(failure)))?;
    if outstanding {
        match finish_retirement(&retirement, retirement_ports, lock) {
            Ok(()) => {}
            Err(FinishFailure::Failed(
                failure @ RetirementFailure::RestoreIncomplete { .. },
            )) => return Err(NotPromoted::RetirementFailed(failure)),
            Err(failure) => return Err(rewind(from_finish(failure))),
        }
    }
    let items = identities(retirement_ports)?;
    let promoted_item = items.iter().find(|item| {
        same(&item.id, key.as_str())
            && item
                .aliases
                .iter()
                .any(|alias| same(alias, record.draft_id.as_str()))
    });
    let next_sync = promoted_item
        .and_then(|item| retirement_ports.files.reader.read(&item.path).ok())
        .flatten()
        .and_then(|content| digest::local(&content).ok())
        .map_or(work::sync::SyncState::Synced, |promoted| {
            baseline.next_sync(&promoted)
        });
    if promoted_item.is_some() {
        record_created_baseline(
            key.as_str(),
            baseline,
            retirement_ports.baseline,
            now_epoch(),
        )
        .map_err(|error| unwritable(Some(key), &error))?;
    }
    records
        .remove(&record.draft_id)
        .map_err(|error| unwritable(Some(key), &error))?;
    Ok(Promotion::Completed(key.clone(), next_sync))
}
