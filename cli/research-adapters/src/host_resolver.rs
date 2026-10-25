//! Resolving a page's host through DNS, bounded as a whole.

use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;

use hickory_resolver::config::ResolverOpts;
use hickory_resolver::TokioResolver;
use network::Host;
use research::sources::web::fetch::HostResolver;
use research::sources::web::fetch::ResolveFailure;

use crate::host_form::ascii_host;

/// A resolver built afresh for each lookup, so no state is shared across
/// researcher processes.
pub struct DnsHostResolver;

impl HostResolver for DnsHostResolver {
    fn resolve(
        &self,
        host: &Host,
        port: u16,
        within: Duration,
    ) -> Result<Vec<SocketAddr>, ResolveFailure> {
        let name = ascii_host(host).ok_or(ResolveFailure::Refused)?;
        bounded_lookup(within, async move {
            let resolver = TokioResolver::builder_tokio()
                .map_err(|_| ResolveFailure::Unresolved)?
                .with_options(resolver_opts(within))
                .build();
            let found = resolver
                .lookup_ip(name)
                .await
                .map_err(|_| ResolveFailure::Unresolved)?;
            Ok(found
                .iter()
                .map(|address| SocketAddr::new(address, port))
                .collect())
        })
    }
}

/// Runs `lookup` to completion within `within` on a runtime that is dropped
/// before this returns, so the blocking HTTP client never runs inside it.
///
/// The bound is over the whole lookup: one `lookup_ip` can send A, AAAA and
/// search-list queries in turn, each bounded only per query by its options.
///
/// # Errors
///
/// [`ResolveFailure::Unresolved`] when the lookup fails, outlives `within`,
/// or no runtime could be built.
pub fn bounded_lookup<T>(
    within: Duration,
    lookup: impl Future<Output = Result<T, ResolveFailure>>,
) -> Result<T, ResolveFailure> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| ResolveFailure::Unresolved)?;
    runtime.block_on(async {
        tokio::time::timeout(within, lookup)
            .await
            .unwrap_or(Err(ResolveFailure::Unresolved))
    })
}

/// Each query is tried once and bounded too, a second limit beneath the
/// whole lookup's.
pub fn resolver_opts(within: Duration) -> ResolverOpts {
    let mut opts = ResolverOpts::default();
    opts.timeout = within;
    opts.attempts = 1;
    opts
}
