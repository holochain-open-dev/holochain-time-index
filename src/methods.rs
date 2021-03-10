use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, Utc};
use hdk3::{hash_path::path::Component, prelude::*};

use crate::entries::{Index, IndexIndex, IndexType, TimeIndex};
use crate::utils::{
    add_time_index_to_path, find_newest_time_path, get_index_for_timestamp, get_next_level_path,
    get_time_path, unwrap_chunk_interval_lock,
};
use crate::EntryChunkIndex;

impl Index {
    /// Create a new time index
    pub(crate) fn new(&self, index: String) -> ExternResult<Path> {
        //These validations are to help zome callers; but should also be present in validation rules
        if self.from > sys_time()? {
            return Err(WasmError::Zome(String::from(
                "Time index cannot start in the future",
            )));
        };
        let max_chunk_interval = unwrap_chunk_interval_lock();
        if self.until - self.from != max_chunk_interval {
            return Err(WasmError::Zome(String::from(
                "Time index should use period equal to max interval set by DNA",
            )));
        };
        if self.from.as_millis() % max_chunk_interval.as_millis() != 0 {
            return Err(WasmError::Zome(String::from(
                "Time index does not follow index interval ordering",
            )));
        };

        let mut time_path = get_time_path(index, self.from)?;
        time_path.push(SerializedBytes::try_from(self)?.bytes().to_owned().into());

        //Create time tree
        let time_path = Path::from(time_path);
        time_path.ensure()?;
        Ok(time_path)
    }

    /// Reads current chunk and moves back N step intervals and tries to get that chunk
    // pub(crate) fn get_previous_chunk(&self, back_steps: u32) -> ExternResult<Option<Index>> {
    //     let max_chunk_interval = unwrap_chunk_interval_lock();
    //     let last_chunk = Index {
    //         from: self.from - (max_chunk_interval * back_steps),
    //         until: self.until - (max_chunk_interval * back_steps),
    //     };
    //     match get(last_chunk.hash()?, GetOptions::content())? {
    //         Some(chunk) => Ok(Some(chunk.entry().to_app_option()?.ok_or(
    //             WasmError::Zome(String::from(
    //                 "Could not deserialize link target into Index",
    //             )),
    //         )?)),
    //         None => Ok(None),
    //     }
    // }

    /// Get current index using sys_time as source for time
    pub fn get_current_index(index: String) -> ExternResult<Option<Path>> {
        //Running with the asumption here that sys_time is always UTC
        let now = sys_time()?;
        let now = DateTime::<Utc>::from_utc(
            NaiveDateTime::from_timestamp(now.as_secs_f64() as i64, now.subsec_nanos()),
            Utc,
        );

        //Create current time path
        let mut time_path = vec![Component::try_from(
            IndexIndex(index).get_sb()?.bytes().to_owned(),
        )?];
        add_time_index_to_path::<TimeIndex>(&mut time_path, &now, IndexType::Year)?;
        add_time_index_to_path::<TimeIndex>(&mut time_path, &now, IndexType::Month)?;
        add_time_index_to_path::<TimeIndex>(&mut time_path, &now, IndexType::Day)?;
        add_time_index_to_path::<TimeIndex>(&mut time_path, &now, IndexType::Hour)?;
        add_time_index_to_path::<TimeIndex>(&mut time_path, &now, IndexType::Minute)?;
        add_time_index_to_path::<TimeIndex>(&mut time_path, &now, IndexType::Second)?;
        let time_path = Path::from(time_path);

        let indexes = time_path.children()?.into_inner();
        let ser_path = indexes
            .clone()
            .into_iter()
            .map(|link| Ok(Index::try_from(Path::try_from(&link.tag)?)?.from))
            .collect::<ExternResult<Vec<Duration>>>()?;
        let permutation = permutation::sort_by(&ser_path[..], |a, b| a.partial_cmp(&b).unwrap());
        let mut ordered_indexes = permutation.apply_slice(&indexes[..]);
        ordered_indexes.reverse();

        match ordered_indexes.pop() {
            Some(link) => match get(link.target, GetOptions::content())? {
                Some(chunk) => Ok(Some(chunk.entry().to_app_option()?.ok_or(
                    WasmError::Zome(String::from("Could not deserialize link target into Index")),
                )?)),
                None => Ok(None),
            },
            None => Ok(None),
        }
    }

    /// Traverses time tree following latest time links until it finds the latest index
    pub fn get_latest_index(index: String) -> ExternResult<Option<Path>> {
        // This should also be smarter. We could at the least derive the index & current year and check that for paths before moving
        // to the previous year. This would help remove 2 get_link() calls from the DHT on source Index path & Index + Year path
        let time_path = Path::from(vec![Component::from(
            IndexIndex(index).get_sb()?.bytes().to_owned(),
        )]);
        let time_path = find_newest_time_path::<TimeIndex>(time_path, IndexType::Year)?;
        let time_path = find_newest_time_path::<TimeIndex>(time_path, IndexType::Month)?;
        let time_path = find_newest_time_path::<TimeIndex>(time_path, IndexType::Day)?;
        let time_path = find_newest_time_path::<TimeIndex>(time_path, IndexType::Hour)?;
        let time_path = find_newest_time_path::<TimeIndex>(time_path, IndexType::Minute)?;

        let indexes = time_path.children()?.into_inner();
        let ser_path = indexes
            .clone()
            .into_iter()
            .map(|link| Ok(Index::try_from(Path::try_from(&link.tag)?)?.from))
            .collect::<ExternResult<Vec<Duration>>>()?;
        let permutation = permutation::sort_by(&ser_path[..], |a, b| a.partial_cmp(&b).unwrap());
        let mut ordered_indexes: Vec<Link> = permutation.apply_slice(&indexes[..]);
        ordered_indexes.reverse();

        match ordered_indexes.pop() {
            Some(link) => match get(link.target, GetOptions::content())? {
                Some(chunk) => Ok(Some(chunk.entry().to_app_option()?.ok_or(
                    WasmError::Zome(String::from("Could not deserialize link target into Index")),
                )?)),
                None => Err(WasmError::Zome(String::from(
                    "Could not deserialize link target into Index",
                ))),
            },
            None => Ok(None),
        }
    }

    /// Get all chunks that exist for some time period between from -> until
    pub(crate) fn get_indexes_for_time_span(
        from: DateTime<Utc>,
        until: DateTime<Utc>,
        index: String,
        link_tag: Option<LinkTag>,
    ) -> ExternResult<Vec<EntryChunkIndex>> {
        //TODO: this is actually super overkill; we dont need to search each part of the time path but instead can derive 
        //path from input where from & until are the same and then only make searches for datetime section where from & until diverge
        let paths = Path::from(vec![Component::from(
            IndexIndex(index).get_sb()?.bytes().to_owned(),
        )]);
        let paths = get_next_level_path(vec![paths], &from, &until, IndexType::Year)?;
        let paths = get_next_level_path(paths, &from, &until, IndexType::Month)?;
        let paths = get_next_level_path(paths, &from, &until, IndexType::Day)?;
        let paths = get_next_level_path(paths, &from, &until, IndexType::Hour)?;
        let paths = get_next_level_path(paths, &from, &until, IndexType::Minute)?;
        //debug!("Got paths after search: {:#?}", paths);
        let mut out: Vec<EntryChunkIndex> = vec![];

        for path in paths {
            let paths = path.children()?.into_inner();
            let mut indexes = paths
                .clone()
                .into_iter()
                .map(|link| {
                    let path = Path::try_from(&link.tag)?;
                    let index = Index::try_from(path.clone())?;
                    let entry_chunk_index = EntryChunkIndex {
                        index: index,
                        links: get_links(path.hash()?, link_tag.clone())?,
                    };
                    Ok(entry_chunk_index)
                })
                .collect::<ExternResult<Vec<EntryChunkIndex>>>()?;
            out.append(&mut indexes);
        }
        out.sort_by(|a, b| a.index.from.partial_cmp(&b.index.from).unwrap());
        out.reverse();

        Ok(out)
    }

    /// Takes a timestamp and creates an index path
    pub(crate) fn create_for_timestamp(index: String, time: DateTime<Utc>) -> ExternResult<Path> {
        let time_index = get_index_for_timestamp(time);
        let path = time_index.new(index)?;
        Ok(path)
    }
}
