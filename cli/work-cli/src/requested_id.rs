//! The ID `work create --id` asks for, admitted before anything is written.

use work::id_scheme::IdScheme;
use work::requested_id::RequestedId;
use work::requested_id::RequestedIdRefusal;
use work::work_item_files::WorkItemFiles as _;
use work_adapters::filesystem::FilesystemWorkItemFiles;

use crate::config::WorkItemLocation;
use crate::create::CreateArgs;
use crate::refusal::Refusal;

/// # Errors
///
/// The admission refusal, which never names a holder.
pub fn admit_requested_id(
    args: &CreateArgs,
    scheme: &IdScheme,
) -> Result<Option<RequestedId>, RequestedIdRefusal> {
    args.id
        .as_deref()
        .map(|id| RequestedId::admit(id, args.project.as_deref(), scheme))
        .transpose()
}

/// Why the taken check did not pass.
pub enum TakenCheckFailure {
    Unlisted(String),
    Taken(RequestedIdRefusal),
}

/// The taken check without the create lock, so it reserves nothing.
///
/// # Errors
///
/// [`TakenCheckFailure::Taken`] naming the holder relative to the
/// repository root, or [`TakenCheckFailure::Unlisted`] when the work
/// directory cannot be listed.
pub fn refuse_if_requested_id_taken(
    requested: &RequestedId,
    location: &WorkItemLocation,
) -> Result<(), TakenCheckFailure> {
    let listing = FilesystemWorkItemFiles::new(&location.work_dir)
        .listing()
        .map_err(|error| TakenCheckFailure::Unlisted(error.to_string()))?;
    requested.refuse_if_taken(&listing).map_err(|refusal| {
        TakenCheckFailure::Taken(refusal.relative_to(&location.root))
    })
}

/// A `--id` refusal as a command reports it.
#[must_use]
pub fn refused_line(refusal: &RequestedIdRefusal) -> Refusal {
    Refusal {
        message: format!("Error: {refusal}"),
        code: crate::exit_codes::of_requested_id_refusal(refusal),
    }
}
