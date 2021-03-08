use hdk3::prelude::*;

use crate::entries::TimeIndex;
use crate::utils::{unwrap_chunk_interval_lock};

impl TimeIndex {
    pub fn validate_chunk(&self) -> ExternResult<()> {
        let max_chunk_interval = unwrap_chunk_interval_lock();
        //TODO: incorrect error type being used here
        if self.from > sys_time()? {
            return Err(WasmError::Zome(String::from(
                "Time chunk cannot start in the future",
            )));
        };
        if self.until - self.from != max_chunk_interval {
            return Err(WasmError::Zome(String::from(
                "Time chunk should use period equal to max interval set by DNA",
            )));
        };
        if self.until - self.from != max_chunk_interval {
            return Err(WasmError::Zome(String::from(
                "Time chunk should use period equal to max interval set by DNA",
            )));
        };
        if self.from.as_millis() % max_chunk_interval.as_millis() != 0 {
            return Err(WasmError::Zome(String::from(
                "Time chunk does not follow chunk interval ordering",
            )));
        };
        Ok(())
    }

    // pub fn validate_chunk_link(&self, link: LinkData) -> ExternResult<()> {
    //     //Interesting interplay developing here
    //     //The complexity to make one link increases with number of links on that chunk
    //     //Thus you could say its worth making chunks as small as possible
    //     //But then you may get added retrieval complexity for a given timeperiod
    //     //I.e having to ask for links on 100 individual second chunks vs two 50 second chunks
    //     //You could probably algorithmically deduce the ideal value for retrival vs commit intensity
    //     if get_links(self.hash(), None).filter(|commited_link| commited_link.author == link.author).count() -1 > DIRECT_CHUNK_LIMIT {
    //         return Err(())
    //     } else {
    //         return Ok(())
    //     }
    // }
}
