use chrono::{DateTime, NaiveDateTime, Utc};
use hdk3::{hash_path::path::Component, prelude::*};

use crate::entries::{
    DayIndex, HourIndex, MinuteIndex, MonthIndex, SecondIndex, TimeChunk, TimeIndex, YearIndex,
};
use crate::utils::{
    add_time_index_to_path, find_newest_time_path, get_time_path, unwrap_chunk_interval_lock, get_chunk_for_timestamp
};

impl TimeChunk {
    /// Create a new chunk & link to time index
    pub(crate) fn create_chunk(&self, index: String) -> ExternResult<()> {
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
        if self.from.as_millis() % max_chunk_interval.as_millis() != 0 {
            return Err(WasmError::Zome(String::from(
                "Time chunk does not follow chunk interval ordering",
            )));
        };

        let mut time_path = get_time_path(index, self.from)?;
        time_path.push(SerializedBytes::try_from(self)?.bytes().to_owned().into());

        //Add time chunk entry to time tree and create
        let time_path = Path::from(time_path);
        time_path.ensure()?;
        Ok(())
    }

    /// Return the hash of the entry
    pub(crate) fn hash(&self) -> ExternResult<EntryHash> {
        hash_entry(self)
    }

    /// Reads current chunk and moves back N step intervals and tries to get that chunk
    pub(crate) fn get_previous_chunk(&self, back_steps: u32) -> ExternResult<Option<TimeChunk>> {
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
    pub fn get_current_chunk(index: String) -> ExternResult<Option<TimeChunk>> {
        //Running with the asumption here that sys_time is always UTC
        let now = sys_time()?;
        let now = DateTime::<Utc>::from_utc(
            NaiveDateTime::from_timestamp(now.as_secs_f64() as i64, now.subsec_nanos()),
            Utc,
        );
        //Create current time path
        //Note here do we want the option to specify the root of the index; i.e being able to create time index over different "anchor" points
        let mut time_path = vec![Component::from(index)];
        add_time_index_to_path::<YearIndex>(&mut time_path, &now, TimeIndex::Year)?;
        add_time_index_to_path::<MonthIndex>(&mut time_path, &now, TimeIndex::Month)?;
        add_time_index_to_path::<DayIndex>(&mut time_path, &now, TimeIndex::Day)?;
        add_time_index_to_path::<HourIndex>(&mut time_path, &now, TimeIndex::Hour)?;
        add_time_index_to_path::<MinuteIndex>(&mut time_path, &now, TimeIndex::Minute)?;
        add_time_index_to_path::<SecondIndex>(&mut time_path, &now, TimeIndex::Second)?;
        let time_path = Path::from(time_path);

        let chunks = get_links(time_path.hash()?, None)?;
        let mut latest_chunk = chunks.into_inner();
        //TODO: this actually needs to read the chunk data vs just link timestamp
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
    pub fn get_latest_chunk(index: String) -> ExternResult<TimeChunk> {
        let time_path = Path::from(vec![Component::from(index)]);

        let time_path = find_newest_time_path::<YearIndex>(time_path, TimeIndex::Year)?;
        let time_path = find_newest_time_path::<MonthIndex>(time_path, TimeIndex::Month)?;
        let time_path = find_newest_time_path::<DayIndex>(time_path, TimeIndex::Day)?;
        let time_path = find_newest_time_path::<HourIndex>(time_path, TimeIndex::Hour)?;
        let time_path = find_newest_time_path::<MinuteIndex>(time_path, TimeIndex::Minute)?;

        let chunks = get_links(time_path.hash()?, None)?;
        let mut latest_chunk = chunks.into_inner();
        //TODO: this actually needs to read the chunk data vs just link timestamp
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
    pub(crate) fn get_chunks_for_time_span(
        _from: DateTime<Utc>,
        _until: DateTime<Utc>,
    ) -> ExternResult<Vec<TimeChunk>> {
        //Check that timeframe specified is greater than the TIME_INDEX_DEPTH.
        //If it is lower then no results will ever be returned
        //Next is to deduce how tree should be traversed and what time index level/path(s)
        //to be used to find chunks
        Ok(vec![])
    }

    /// Gets all links for a given chunk and recurses into any linked lists on chunk
    /// Note for now linked list recursion will not occur
    pub (crate) fn get_links(&self, link_tag: Option<LinkTag>, _limit: Option<usize>) -> ExternResult<Vec<EntryHash>> {
        Ok(get_links(self.hash()?, link_tag)?.into_inner().into_iter().map(|val| val.target).collect())
    }

    pub (crate) fn add_link<T: Into<LinkTag>>(&self, target: EntryHash, link_tag: T) -> ExternResult<()> {
        create_link(self.hash()?, target, link_tag)?;
        Ok(())
    }

    /// Takes a timestamp and creates a chunk that can be used for indexing at given timestamp
    pub (crate) fn create_for_timestamp(index: String, time: DateTime<Utc>) -> ExternResult<TimeChunk> {
        let chunk = get_chunk_for_timestamp(time);
        chunk.create_chunk(index)?;
        Ok(chunk)
    }
}