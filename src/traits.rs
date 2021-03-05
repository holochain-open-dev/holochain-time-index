use chrono::{DateTime, Utc};
use hdk3::prelude::LinkTag;

pub trait EntryTimeIndex {
    ///Time that entry type this trait is implemented on should be indexed under
    fn entry_time(&self) -> DateTime<Utc>;
}
