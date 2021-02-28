use crate::Report;

use async_trait::async_trait;

pub(crate) mod async_cache;
pub(crate) mod sync_cache;
pub(crate) mod sync_segmented;
pub(crate) mod unsync_cache;

pub trait CacheSet<T> {
    fn process(&mut self, entry: &T, report: &mut Report);
}

#[async_trait]
pub trait AsyncCacheSet<T> {
    async fn process(&mut self, entry: &T, report: &mut Report);
}
