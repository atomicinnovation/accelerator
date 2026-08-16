//! The remote sync state machine: classify, decide, label and plan, over
//! the `tracker::RemoteTracker` port. No filesystem, no subprocess, no
//! tracker calls — those live in `work-adapters`.

pub mod classify;
pub mod clock;
pub mod decide;
pub mod label;
pub mod plan;
pub mod push_decide;
pub mod push_precondition;
pub mod state;

pub use crate::sync::classify::classify;
pub use crate::sync::classify::BaselineEntry;
pub use crate::sync::classify::ItemDigests;
pub use crate::sync::classify::RemotePresence;
pub use crate::sync::classify::Subject;
pub use crate::sync::clock::RunClock;
pub use crate::sync::decide::decide;
pub use crate::sync::decide::resolve_conflict_token;
pub use crate::sync::decide::Action;
pub use crate::sync::decide::Dirtiness;
pub use crate::sync::decide::Resolution;
pub use crate::sync::decide::SyncDirection;
pub use crate::sync::label::classify_external_id;
pub use crate::sync::label::label;
pub use crate::sync::label::RenderableState;
pub use crate::sync::label::SyncPresence;
pub use crate::sync::plan::needs_body_read;
pub use crate::sync::plan::plan;
pub use crate::sync::plan::PlanInput;
pub use crate::sync::plan::PlannedAction;
pub use crate::sync::plan::RemoteFacts;
pub use crate::sync::plan::SyncPlan;
pub use crate::sync::push_decide::push_decide;
pub use crate::sync::push_decide::PushOutcome;
pub use crate::sync::push_precondition::push_precondition;
pub use crate::sync::push_precondition::MarkerState;
pub use crate::sync::push_precondition::PendingPush;
pub use crate::sync::push_precondition::PushPrecondition;
pub use crate::sync::push_precondition::RefusalReason;
pub use crate::sync::push_precondition::RequestFingerprint;
pub use crate::sync::state::SyncState;
