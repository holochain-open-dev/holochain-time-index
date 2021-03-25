use chrono::{DateTime, Utc};
use hdk::prelude::{EntryHash, ExternResult};

pub trait IndexableEntry {
    ///Time that entry type this trait is implemented on should be indexed under
    fn entry_time(&self) -> DateTime<Utc>;
    fn hash(&self) -> ExternResult<EntryHash>;
}
