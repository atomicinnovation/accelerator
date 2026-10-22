//! The one bundle of ports a cache root's resolvers and retention share, so a
//! single recorder sees every call any of them makes.

use std::sync::Arc;

use crate::launch::core::tree::Clock;

use super::super::filesystem::CacheFilesystem;
use super::super::ownership::Ownership;

pub struct CachePorts {
    pub filesystem: Arc<dyn CacheFilesystem + Send + Sync>,
    pub ownership: Arc<dyn Ownership + Send + Sync>,
    pub clock: Arc<dyn Clock + Send + Sync>,
}
