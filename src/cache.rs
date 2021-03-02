use crate::Report;

use async_trait::async_trait;

pub(crate) mod async_cache;
pub(crate) mod sync_cache;
pub(crate) mod sync_segmented;
pub(crate) mod unsync_cache;

pub trait CacheSet<T> {
    fn get_or_insert(&mut self, entry: &T, report: &mut Report);
    fn invalidate(&mut self, entry: &T);
    fn invalidate_all(&mut self);
}

#[async_trait]
pub trait AsyncCacheSet<T> {
    async fn get_or_insert(&mut self, entry: &T, report: &mut Report);
    async fn invalidate(&mut self, entry: &T);
    fn invalidate_all(&mut self);
}
