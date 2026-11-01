//! The ID `work create --id` asks for, admitted before anything is written.

use work::id_scheme::WorkItemIdScheme;
use work::requested_id::RequestedId;

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
