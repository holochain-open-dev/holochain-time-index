use hdk::prelude::*;
use std::convert::Infallible;

#[derive(thiserror::Error, Debug)]
pub enum IndexError {
    #[error(transparent)]
    Serialization(#[from] SerializedBytesError),
    #[error(transparent)]
    Infallible(#[from] Infallible),
    #[error(transparent)]
    EntryError(#[from] EntryError),
    #[error(transparent)]
    Wasm(#[from] WasmError),
    #[error("Internal Error. Error: {0}")]
    InternalError(&'static str),
    // #[error(transparent)]
    // HdkError(#[from] HdkError),
    #[error("Invalid Request Data. Error: {0}")]
    RequestError(&'static str),
}

pub type IndexResult<T> = Result<T, IndexError>;

impl From<IndexError> for String {
    fn from(e: IndexError) -> Self {
        format!("{}", e)
    }
}
