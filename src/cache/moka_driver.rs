use async_trait::async_trait;
use thiserror::Error;

use crate::{parser::TraceEntry, Report};

pub(crate) mod async_cache;
pub(crate) mod sync_cache;
pub(crate) mod sync_segmented;

pub(crate) trait GetOrInsertOnce {
    fn get_or_insert_once(&self, entry: &TraceEntry, report: &mut Report);
}

#[async_trait]
trait AsyncGetOrInsertOnce {
    async fn get_or_insert_once(&self, entry: &TraceEntry, report: &mut Report);
}

// https://rust-lang.github.io/rust-clippy/master/index.html#enum_variant_names
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy)]
enum InitClosureType {
    GetOrInsert,
    GetOrTryInsertWithError1,
    GetOrTyyInsertWithError2,
}

impl InitClosureType {
    fn select(block: usize) -> Self {
        match block % 4 {
            0 => Self::GetOrTryInsertWithError1,
            1 => Self::GetOrTyyInsertWithError2,
            _ => Self::GetOrInsert,
        }
    }
}

#[derive(Debug, Error)]
#[error("init closure failed with error one")]
struct InitClosureError1;

#[derive(Debug, Error)]
#[error("init closure failed with error two")]
struct InitClosureError2;
