use chrono::NaiveDateTime;
use std::{convert::TryInto, ops::Sub};

use hdk::prelude::SerializedBytes;

use crate::entries::{Index, StringIndex, TimeIndex};
use crate::errors::IndexResult;

/// Helper function to get serializedbytes of StringIndex and make this cleaner in the code
impl StringIndex {
    pub fn get_sb(self) -> IndexResult<SerializedBytes> {
        Ok(self.try_into()?)
    }
}

/// Helper function to get serializedbytes of TimeIndex and make this cleaner in the code
impl TimeIndex {
    pub fn get_sb(self) -> IndexResult<SerializedBytes> {
        Ok(self.try_into()?)
    }
}

impl std::fmt::Debug for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("Index");
        debug_struct.field("from", &self.from.as_secs().to_string());
        debug_struct.field("until", &self.until.as_secs().to_string());
        debug_struct.field("diff", &self.until.sub(self.from));
        debug_struct.field(
            "timestamp",
            &NaiveDateTime::from_timestamp_opt(self.from.as_secs() as i64, self.from.subsec_nanos()).unwrap(),
        );
        debug_struct.field(
            "timestamp_until",
            &NaiveDateTime::from_timestamp_opt(self.until.as_secs() as i64, self.until.subsec_nanos()).unwrap(),
        );
        debug_struct.finish()
    }
}
