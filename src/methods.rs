use chrono::{DateTime, NaiveDateTime, Utc};
use hdk3::{hash_path::path::Component, prelude::*};

use crate::entries::{
    DayIndex, HourIndex, MinuteIndex, MonthIndex, SecondIndex, TimeChunk, TimeIndex, YearIndex,
};
use crate::utils::{
    add_time_index_to_path, find_newest_time_path, get_time_path, unwrap_chunk_interval_lock,
    unwrap_genesis_chunk,
};

impl TimeChunk {
    /// Create a new chunk & link to time index
    pub(crate) fn create_chunk(&self) -> ExternResult<()> {
        //These validations are to help zome callers; but should also be present in validation rules
        if self.from > sys_time()? {
            return Err(WasmError::Zome(String::from(
                "Time chunk cannot start in the future",
            )));
        };
        let max_chunk_interval = unwrap_chunk_interval_lock();
        if self.until - self.from != max_chunk_interval {
            return Err(WasmError::Zome(String::from(
                "Time chunk should use period equal to max interval set by DNA",
            )));
        };
        //Genesis should actually be embedded into DHT via properties; this saves lookup on each insert/validation
        let genesis = unwrap_genesis_chunk();
        if (self.from - genesis.from).as_millis() % max_chunk_interval.as_millis() != 0 {
            return Err(WasmError::Zome(String::from(
                "Time chunk does not follow chunk interval ordering since genesis",
            )));
        };

        let time_path = get_time_path(self.from)?;

        //Create the TimeChunk entry
        create_entry(self)?;

        //Link TimeChunk entry to time tree
        let time_path = Path::from(time_path);
        time_path.ensure()?;
        create_link(time_path.hash()?, self.hash()?, LinkTag::new("chunk"))?;
        Ok(())
    }

    /// Return the hash of the entry
    pub fn hash(&self) -> ExternResult<EntryHash> {
        hash_entry(self)
    }

    /// Reads current chunk and moves back N step intervals and tries to get that chunk
    pub fn get_previous_chunk(&self, back_steps: u32) -> ExternResult<Option<TimeChunk>> {
        let max_chunk_interval = unwrap_chunk_interval_lock();
        let last_chunk = TimeChunk {
            from: self.from - (max_chunk_interval * back_steps),
            until: self.until - (max_chunk_interval * back_steps),
        };
        match get(last_chunk.hash()?, GetOptions::content())? {
            Some(chunk) => Ok(Some(chunk.entry().to_app_option()?.ok_or(
                WasmError::Zome(String::from(
                    "Could not deserialize link target into TimeChunk",
                )),
            )?)),
            None => Ok(None),
        }
    }

    /// Get current chunk using sys_time as source for time
    pub fn get_current_chunk() -> ExternResult<Option<TimeChunk>> {
        //Running with the asumption here that sys_time is always UTC
        let now = sys_time()?;
        let now = DateTime::<Utc>::from_utc(
            NaiveDateTime::from_timestamp(now.as_secs_f64() as i64, now.subsec_nanos()),
            Utc,
        );
        //Create current time path
        let mut time_path = vec![Component::from("time")];
        add_time_index_to_path::<YearIndex>(&mut time_path, &now, TimeIndex::Year)?;
        add_time_index_to_path::<MonthIndex>(&mut time_path, &now, TimeIndex::Month)?;
        add_time_index_to_path::<DayIndex>(&mut time_path, &now, TimeIndex::Day)?;
        add_time_index_to_path::<HourIndex>(&mut time_path, &now, TimeIndex::Hour)?;
        add_time_index_to_path::<MinuteIndex>(&mut time_path, &now, TimeIndex::Minute)?;
        add_time_index_to_path::<SecondIndex>(&mut time_path, &now, TimeIndex::Second)?;
        let time_path = Path::from(time_path);

        let chunks = get_links(time_path.hash()?, Some(LinkTag::new("chunk")))?;
        let mut latest_chunk = chunks.into_inner();
        latest_chunk.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap());

        match latest_chunk.pop() {
            Some(link) => match get(link.target, GetOptions::content())? {
                Some(chunk) => Ok(Some(chunk.entry().to_app_option()?.ok_or(
                    WasmError::Zome(String::from(
                        "Could not deserialize link target into TimeChunk",
                    )),
                )?)),
                None => Ok(None),
            },
            None => Ok(None),
        }
    }

    /// Traverses time tree following latest time links until it finds the latest chunk
    pub fn get_latest_chunk() -> ExternResult<TimeChunk> {
        let time_path = Path::from(vec![Component::from("time")]);

        let time_path = find_newest_time_path::<YearIndex>(time_path, TimeIndex::Year)?;
        let time_path = find_newest_time_path::<MonthIndex>(time_path, TimeIndex::Month)?;
        let time_path = find_newest_time_path::<DayIndex>(time_path, TimeIndex::Day)?;
        let time_path = find_newest_time_path::<HourIndex>(time_path, TimeIndex::Hour)?;
        let time_path = find_newest_time_path::<MinuteIndex>(time_path, TimeIndex::Minute)?;

        let chunks = get_links(time_path.hash()?, Some(LinkTag::new("chunk")))?;
        let mut latest_chunk = chunks.into_inner();
        latest_chunk.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap());

        match latest_chunk.pop() {
            Some(link) => match get(link.target, GetOptions::content())? {
                Some(chunk) => {
                    Ok(chunk
                        .entry()
                        .to_app_option()?
                        .ok_or(WasmError::Zome(String::from(
                            "Could not deserialize link target into TimeChunk",
                        )))?)
                }
                None => Err(WasmError::Zome(String::from(
                    "Could not deserialize link target into TimeChunk",
                ))),
            },
            None => Err(WasmError::Zome(String::from(
                "Expected a chunk on time path",
            ))),
        }
    }

    /// Get all chunks that exist for some time period between from -> until
    pub fn get_chunks_for_time_span(
        _from: DateTime<Utc>,
        _until: DateTime<Utc>,
    ) -> ExternResult<Vec<EntryHash>> {
        //Check that timeframe specified is greater than the TIME_INDEX_DEPTH.
        //If it is lower then no results will ever be returned
        //Next is to deduce how tree should be traversed and what time index level/path(s)
        //to be used to find chunks
        Ok(vec![])
    }

    pub fn add_link(&self, _target: EntryHash) -> ExternResult<()> {
        //TODO
        //Read how many links an agent already has on a given chunk
        //If under DIRECT_CHUNK_LINK_LIMIT then make direct link
        //otherwise create linked list starting from latest link or latest link in chain of links
        Ok(())
    }

    pub fn get_links(&self, _limit: u32) -> ExternResult<Vec<EntryHash>> {
        //TODO
        //Read for direct links on chunk as well as traverse into any linked list on a chunk to find
        //any other linked addresses
        Ok(vec![])
    }

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
        //Genesis should actually be embedded into DHT via properties; this saves lookup on each insert/validation
        let genesis = unwrap_genesis_chunk();
        if (self.from - genesis.from).as_millis() % max_chunk_interval.as_millis() != 0 {
            return Err(WasmError::Zome(String::from(
                "Time chunk does not follow chunk interval ordering since genesis",
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

// /// Will take current time and try to find a chunk that fits; if no chunk is found then it will create a chunk that fits
// pub fn get_valid_chunk() -> ExternResult<TimeChunk> {
//     //TODO:
//     //determine how many hops from genesis until value until we would get a chunk where the
//     //current time would fit inside the from & until of chunk. Try to get this chunk
//     //if it does not exist then create it and return other wise just return retrieved chunk
//     Ok(TimeChunk)
// }
