//! What every launcher start does before dispatch: identify the cache root
//! once, choose its retention, and claim the running version, then build each
//! resolver over that one root and one bundle of ports.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::launch::core::ResolutionError;
use crate::launch::outbound::resolve::cache_root::{
    identify, CacheRoot, CacheRootConfig,
};
use crate::launch::outbound::resolve::fetcher::Fetcher;
use crate::launch::outbound::resolve::keys::TrustedKeys;
use crate::launch::outbound::resolve::retention::launcher_id;
use crate::launch::outbound::resolve::retention::ports::CachePorts;
use crate::launch::outbound::resolve::retention::root_retention::{
    for_root, RootRetention,
};
use crate::launch::outbound::resolve::tree::lease::LivenessProbe;
use crate::launch::outbound::resolve::tree::{
    ExpectedDigests, StepObserver, TreeResolver,
};
use crate::launch::outbound::resolve::{
    FetchVerifyCacheResolver, ResolverConfig, HOST_PLATFORM,
};

pub struct StartUp {
    /// An identification error is carried rather than raised, so a built-in
    /// that never touches the cache behaves as if the root were never read.
    cache_root: Result<CacheRoot, String>,
    retention: Arc<dyn RootRetention + Send + Sync>,
    ports: Arc<CachePorts>,
}

/// What a dispatch resolver needs beyond the cache root and the ports.
pub struct DispatchConfig {
    pub base_url: String,
    pub keys: TrustedKeys,
    pub fetcher: Fetcher,
}

/// What a tree resolver borrows from its caller, who builds it lazily.
pub struct TreeConfig<'a> {
    pub base_url: String,
    pub keys: &'a TrustedKeys,
    pub fetcher: &'a Fetcher,
    pub liveness: &'a dyn LivenessProbe,
    pub expected_digests: ExpectedDigests,
    pub waiter_bound: Duration,
    pub steps: &'a dyn StepObserver,
}

/// Identify the root, then claim the running version in it before anything
/// can look an entry up.
#[must_use]
pub fn start_up(config: &CacheRootConfig, ports: Arc<CachePorts>) -> StartUp {
    let cache_root = identify(config, ports.filesystem.as_ref());
    let retention = for_root(cache_root.as_ref().ok(), Arc::clone(&ports));
    retention.claim_running_version();
    StartUp {
        cache_root: cache_root.map_err(|error| match error {
            ResolutionError::CacheRootUnavailable { detail } => detail,
            other => other.to_string(),
        }),
        retention,
        ports,
    }
}

impl StartUp {
    /// # Errors
    ///
    /// [`ResolutionError::CacheRootUnavailable`] when the root could not be
    /// identified.
    pub fn dispatch_resolver(
        &self,
        config: DispatchConfig,
    ) -> Result<FetchVerifyCacheResolver, ResolutionError> {
        Ok(FetchVerifyCacheResolver::with_fetcher(
            ResolverConfig::production(config.base_url, self.cache_root()?),
            config.keys,
            config.fetcher,
            Arc::clone(&self.ports.filesystem),
        ))
    }

    /// # Errors
    ///
    /// [`ResolutionError::CacheRootUnavailable`] when the root could not be
    /// identified.
    pub fn tree_resolver<'a>(
        &'a self,
        config: TreeConfig<'a>,
    ) -> Result<TreeResolver<'a>, ResolutionError> {
        Ok(TreeResolver {
            cache_root: self.cache_root()?,
            base_url: config.base_url,
            platform: HOST_PLATFORM.to_owned(),
            expected_version: env!("CARGO_PKG_VERSION").to_owned(),
            keys: config.keys,
            fetcher: config.fetcher,
            clock: self.ports.clock.as_ref(),
            liveness: config.liveness,
            launcher_id: launcher_id(),
            expected_digests: config.expected_digests,
            waiter_bound: config.waiter_bound,
            steps: config.steps,
        })
    }

    #[must_use]
    pub fn retention(&self) -> &dyn RootRetention {
        self.retention.as_ref()
    }

    fn cache_root(&self) -> Result<PathBuf, ResolutionError> {
        self.cache_root
            .as_ref()
            .map(|root| root.path.clone())
            .map_err(|detail| ResolutionError::CacheRootUnavailable {
                detail: detail.clone(),
            })
    }
}
