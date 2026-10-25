//! One `research fetch web metadata` call: a page's declared metadata,
//! fetched within the call's deadline. It reads no project, configuration or
//! credential.

use research::sources::fetch::Clock;
use research::sources::metadata::SourceMetadata;
use research::sources::request::WebRequest;
use research::sources::schedule::Deadline;
use research::sources::web::fetch::fetch_page_metadata;
use research::sources::web::fetch::HostResolver;
use research::sources::web::fetch::PageTransport;
use research::sources::web::fetch::WebFailure;
use research::sources::web::fetch::WebPorts;
use research_adapters::web_html::WebHtmlDecoder;

pub struct WebAdapters {
    pub resolver: Box<dyn HostResolver>,
    pub transport: Box<dyn PageTransport>,
}

/// # Errors
///
/// The [`WebFailure`] that ended the call before a page arrived.
pub fn run(
    request: &WebRequest,
    adapters: &WebAdapters,
    clock: &dyn Clock,
    deadline: &Deadline,
) -> Result<SourceMetadata, WebFailure> {
    let WebRequest::Metadata(url) = request;
    fetch_page_metadata(
        url,
        deadline,
        &WebPorts {
            resolver: adapters.resolver.as_ref(),
            transport: adapters.transport.as_ref(),
            decoder: &WebHtmlDecoder,
            clock,
        },
    )
}
