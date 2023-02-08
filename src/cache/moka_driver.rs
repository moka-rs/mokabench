use thiserror::Error;

pub(crate) mod async_cache;
pub(crate) mod sync_cache;
pub(crate) mod sync_segmented;

// https://rust-lang.github.io/rust-clippy/master/index.html#enum_variant_names
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum InitClosureType {
    GetOrInsert,
    GetOrTryInsertWithError1,
    GetOrTyyInsertWithError2,
}

impl InitClosureType {
    pub(crate) fn select(block: usize) -> Self {
        match block % 4 {
            0 => Self::GetOrTryInsertWithError1,
            1 => Self::GetOrTyyInsertWithError2,
            _ => Self::GetOrInsert,
        }
    }
}

#[derive(Debug, Error)]
#[error("init closure failed with error one")]
pub(crate) struct InitClosureError1;

#[derive(Debug, Error)]
#[error("init closure failed with error two")]
pub(crate) struct InitClosureError2;
