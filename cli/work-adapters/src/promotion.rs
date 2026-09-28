//! Promoting a draft onto the issue a tracker creates for it.
//!
//! The service creates the remote issue, carries the tracker key into the
//! issue's H1, retires the draft ID to the key, and records the sync
//! baseline, recording each stage before the next, so any run can finish a
//! promotion another left.

use std::path::PathBuf;

use corpus::StoreError;
use tracker::ExternalId;
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
use work::sync::RequestFingerprint;

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
    let (frontmatter, body) = digest::split_frontmatter_and_body(&content)
        .map_err(|error| unreadable(error.to_string()))?;
    let field =
        |key| work::show::read_field_raw(&frontmatter, key).unwrap_or_default();
    Ok(DraftContent {
        path: item.path.clone(),
        title: field("title"),
        kind: field("kind"),
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
            PromotionStep::RetitleRemote(key) => {
                promotion.retitle_remote(&key, &stored)?;
            }
            PromotionStep::Retire(key) => {
                return promotion
                    .retire(&key, &stored)
                    .map(Promotion::Completed);
            }
            PromotionStep::Finish => {
                let StoredRecord::Present(record) = stored else {
                    return Err(NotPromoted::CreateOutcomeUnknown);
                };
                let key = finish_promotion(
                    &record,
                    ports.retirement,
                    ports.records,
                    &lock,
                )?;
                return Ok(Promotion::Completed(key));
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
        return Err(NotPromoted::Refused(
            work::retirement::RetirementRefusal::ItemNotFound(
                draft.as_str().to_owned(),
            ),
        ));
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

    fn create_issue(&self) -> Result<(), NotPromoted> {
        let content = self.content;
        let request = RequestFingerprint {
            title: content.title.clone(),
            digest: pending_push::request_digest(
                &content.title,
                &content.body,
                &content.kind,
            ),
            attempted_at: now_epoch(),
            failure: None,
        };
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
    ) -> Result<ExternalId, NotPromoted> {
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
/// item, then removes the record.
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
) -> Result<ExternalId, NotPromoted> {
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
    let settled = identities(retirement_ports)?.iter().any(|item| {
        same(&item.id, key.as_str())
            && item
                .aliases
                .iter()
                .any(|alias| same(alias, record.draft_id.as_str()))
    });
    if settled {
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
    Ok(key.clone())
}
