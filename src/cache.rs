use crate::Report;

use async_trait::async_trait;

pub(crate) mod async_cache;
pub(crate) mod sync_cache;
pub(crate) mod sync_segmented;
pub(crate) mod unsync_cache;

pub trait CacheSet<T> {
    fn get_or_insert(&mut self, entry: &T, report: &mut Report);
    fn get_or_insert_once(&mut self, entry: &T, report: &mut Report);
    fn invalidate(&mut self, entry: &T);
    fn invalidate_all(&mut self);
    fn invalidate_entries_if(&mut self, entry: &T);
}

#[async_trait]
pub trait AsyncCacheSet<T> {
    async fn get_or_insert(&mut self, entry: &T, report: &mut Report);
    async fn get_or_insert_once(&mut self, entry: &T, report: &mut Report);
    async fn invalidate(&mut self, entry: &T);
    fn invalidate_all(&mut self);
    fn invalidate_entries_if(&mut self, entry: &T);
}

#[derive(Default)]
pub(crate) struct Counters {
    insert_count: usize,
    read_count: usize,
}

impl Counters {
    pub(crate) fn inserted(&mut self) {
        self.insert_count += 1;
        self.read_count += 1;
    }

    pub(crate) fn read(&mut self) {
        self.read_count += 1;
    }

    pub(crate) fn add_to_report(&self, report: &mut Report) {
        report.insert_count += self.insert_count;
        report.read_count += self.read_count;
        report.hit_count += self.read_count - self.insert_count;
    }
}

const VALUE_LEN: usize = 256;

pub(crate) fn make_value(key: usize) -> Box<[u8]> {
    let mut value = vec![0; VALUE_LEN].into_boxed_slice();
    value[0] = (key % 256) as u8;
    value
}
