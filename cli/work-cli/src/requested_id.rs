//! The ID `work create --id` asks for, admitted before anything is written.

use work::id_scheme::WorkItemIdScheme;
use work::requested_id::RequestedId;
use work::work_item_files::WorkItemFiles as _;
use work_adapters::filesystem::FilesystemWorkItemFiles;

use crate::config::WorkItemLocation;
use crate::create::CreateArgs;

/// # Errors
///
/// The admission refusal's message, which never names a holder.
pub fn admit_requested_id(
    args: &CreateArgs,
    scheme: &WorkItemIdScheme,
) -> Result<Option<RequestedId>, String> {
    args.id
        .as_deref()
        .map(|id| RequestedId::admit(id, args.project.as_deref(), scheme))
        .transpose()
        .map_err(|refusal| refusal.to_string())
}

/// The taken check without the create lock, so it reserves nothing.
///
/// # Errors
///
/// `E_ID_TAKEN` naming the holder relative to the repository root, or a
/// message when the work directory cannot be listed.
pub fn refuse_if_requested_id_taken(
    requested: &RequestedId,
    location: &WorkItemLocation,
) -> Result<(), String> {
    let listing = FilesystemWorkItemFiles::new(&location.work_dir)
        .listing()
        .map_err(|error| error.to_string())?;
    requested
        .refuse_if_taken(&listing)
        .map_err(|refusal| refusal.relative_to(&location.root).to_string())
}
