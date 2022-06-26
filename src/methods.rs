use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, Utc};
use hdk::{hash_path::path::Component, prelude::*};

use crate::bfs::find_paths_for_time_span;
use crate::dfs::methods::make_dfs_search;
use crate::search::find_newest_time_path;
use crate::utils::{add_time_index_to_path, get_index_for_timestamp, get_time_path};
use crate::{
    entries::{Index, IndexType, StringIndex, TimeIndex},
    EntryChunkIndex, IndexableEntry, SearchStrategy, MAX_CHUNK_INTERVAL,
    LinkTypes,
};
use crate::{
    errors::{IndexError, IndexResult},
    Order,
};

impl Index {
    /// Create a new time index
    pub(crate) fn new(&self, index: String) -> IndexResult<Path> {
        //These validations are to help zome callers; but should also be present in validation rules
        let now_since_epoch = sys_time()?
            .checked_difference_signed(&Timestamp::from_micros(0))
            .ok_or(IndexError::InternalError("Should not overflow"))?
            .to_std()
            .map_err(|_err| IndexError::InternalError("Should not overflow"))?;
        if self.from > now_since_epoch {
            return Err(IndexError::RequestError(
                "Time index cannot start in the future",
            ));
        };
        if self.until - self.from != *MAX_CHUNK_INTERVAL {
            return Err(IndexError::RequestError(
                "Time index should use period equal to max interval set by DNA",
            ));
        };
        if self.from.as_millis() % MAX_CHUNK_INTERVAL.as_millis() != 0 {
            return Err(IndexError::RequestError(
                "Time index does not follow index interval ordering",
            ));
        };

        let mut time_path = get_time_path(index, self.from)?;
        time_path.push(SerializedBytes::try_from(self)?.bytes().to_owned().into());

        //Create time tree
        let time_path = Path::from(time_path).typed(LinkTypes::PathLink)?;
        time_path.ensure()?;
        Ok(time_path.path)
    }
}

/// Get current index using sys_time as source for time
pub fn get_current_index(index: String) -> IndexResult<Option<Path>> {
    //Running with the asumption here that sys_time is always UTC
    let now = sys_time()?.as_seconds_and_nanos();
    let now = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(now.0, now.1), Utc);

    //Create current time path
    let mut time_path = vec![Component::try_from(
        StringIndex(index).get_sb()?.bytes().to_owned(),
    )?];
    add_time_index_to_path::<TimeIndex>(&mut time_path, &now, IndexType::Year)?;
    add_time_index_to_path::<TimeIndex>(&mut time_path, &now, IndexType::Month)?;
    add_time_index_to_path::<TimeIndex>(&mut time_path, &now, IndexType::Day)?;
    add_time_index_to_path::<TimeIndex>(&mut time_path, &now, IndexType::Hour)?;
    add_time_index_to_path::<TimeIndex>(&mut time_path, &now, IndexType::Minute)?;
    add_time_index_to_path::<TimeIndex>(&mut time_path, &now, IndexType::Second)?;
    let time_path = Path::from(time_path).typed(LinkTypes::PathLink)?;

    let indexes = time_path.children_paths()?;
    let ser_path = indexes
        .clone()
        .into_iter()
        .map(|path| Ok(Index::try_from(path.path)?.from))
        .collect::<IndexResult<Vec<Duration>>>()?;
    let permutation = permutation::sort_by(&ser_path[..], |a, b| a.partial_cmp(&b).unwrap());
    let mut ordered_indexes = permutation.apply_slice(&indexes[..]);
    ordered_indexes.reverse();

    match ordered_indexes.pop() {
        Some(link) => match get(link.path_entry_hash()?, GetOptions::latest())? {
            Some(chunk) => Ok(Some(chunk.entry().to_app_option()?.ok_or(
                IndexError::InternalError("Expected element to contain app entry data"),
            )?)),
            None => Ok(None),
        },
        None => Ok(None),
    }
}

/// Traverses time tree following latest time links until it finds the latest index
pub fn get_latest_index(index: String) -> IndexResult<Option<Path>> {
    // This should also be smarter. We could at the least derive the index & current year and check that for paths before moving
    // to the previous year. This would help remove 2 get_link() calls from the DHT on source Index path & Index + Year path
    let time_path = Path::from(vec![Component::from(
        StringIndex(index).get_sb()?.bytes().to_owned(),
    )]);
    let time_path = find_newest_time_path::<TimeIndex>(time_path, IndexType::Year)?;
    let time_path = find_newest_time_path::<TimeIndex>(time_path, IndexType::Month)?;
    let time_path = find_newest_time_path::<TimeIndex>(time_path, IndexType::Day)?;
    let time_path = find_newest_time_path::<TimeIndex>(time_path, IndexType::Hour)?;
    let time_path = find_newest_time_path::<TimeIndex>(time_path, IndexType::Minute)?;
    let time_path = find_newest_time_path::<TimeIndex>(time_path, IndexType::Second)?;

    let indexes = time_path.typed(LinkTypes::PathLink)?.children_paths()?.into_iter().map(|val| val.path).collect::<Vec<_>>();
    let ser_path = indexes
        .clone()
        .into_iter()
        .map(|path| Ok(Index::try_from(path)?.from))
        .collect::<IndexResult<Vec<Duration>>>()?;
    let permutation = permutation::sort_by(&ser_path[..], |a, b| a.partial_cmp(&b).unwrap());
    let mut ordered_indexes: Vec<Path> = permutation.apply_slice(&indexes[..]);
    ordered_indexes.reverse();

    //TODO: dont error out if cant find link target; just use next link
    match ordered_indexes.pop() {
        Some(link) => match get(link.path_entry_hash()?, GetOptions::latest())? {
            Some(chunk) => Ok(Some(chunk.entry().to_app_option()?.ok_or(
                IndexError::InternalError("Expected element to contain app entry data"),
            )?)),
            None => Err(IndexError::InternalError(
                "Expected link target to contain point to an entry",
            )),
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
) -> IndexResult<Vec<EntryChunkIndex>> {
    let paths = find_paths_for_time_span(from, until, index)?;
    //debug!("Got paths after search: {:#?}", paths);
    let mut out: Vec<EntryChunkIndex> = vec![];

    for path in paths {
        let paths = path.typed(LinkTypes::PathLink)?.children_paths()?;
        let mut indexes = paths
            .clone()
            .into_iter()
            .map(|path| {
                let index = Index::try_from(path.path.clone())?;
                let entry_chunk_index = EntryChunkIndex {
                    index: index,
                    links: get_links(path.path_entry_hash()?, LinkTypes::Index, link_tag.clone())?,
                };
                Ok(entry_chunk_index)
            })
            .collect::<IndexResult<Vec<EntryChunkIndex>>>()?;
        out.append(&mut indexes);
    }
    //NOTE: untested logic
    let timestamps = out
        .clone()
        .into_iter()
        .map(|val| val.index.from)
        .collect::<Vec<Duration>>();
    let permutation = permutation::sort_by(&timestamps[..], |a, b| a.partial_cmp(&b).unwrap());
    let mut ordered_indexes: Vec<EntryChunkIndex> = permutation.apply_slice(&out[..]);
    ordered_indexes.reverse();

    Ok(ordered_indexes)
}

/// Get all links that exist for some time period between from -> until
pub(crate) fn get_links_for_time_span(
    index: String,
    from: DateTime<Utc>,
    until: DateTime<Utc>,
    link_tag: Option<LinkTag>,
    limit: Option<usize>,
) -> IndexResult<Vec<Link>> {
    let order = if from > until {
        Order::Desc
    } else {
        Order::Asc
    };

    if limit.is_some() {
        debug!("hc_time_index::get_links_for_time_span: WARNING: Limit not supported on Bfs strategy. All links between bounds will be retrieved and returned");
    };
    let paths = find_paths_for_time_span(from, until, index)?;
    //debug!("Got paths after search: {:#?}", paths);
    let mut out: Vec<Link> = vec![];
    for path in paths {
        let paths = path.typed(LinkTypes::PathLink)?.children_paths()?;
        let mut indexes = paths
            .clone()
            .into_iter()
            .map(|path| {
                let links = get_links(path.path_entry_hash()?, LinkTypes::Index, link_tag.clone())?;
                Ok(links)
            })
            .collect::<IndexResult<Vec<Vec<Link>>>>()?
            .into_iter()
            .flatten()
            .collect();
        out.append(&mut indexes);
    }
    //TODO: do sort based on path value
    match order {
        Order::Desc => {
            out.sort_by(|a, b| b.timestamp.partial_cmp(&a.timestamp).unwrap());
        }
        Order::Asc => {
            out.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap());
        }
    }
    Ok(out)
}

/// Get all links that exist for some time period between from -> until
pub(crate) fn get_links_and_load_for_time_span<
    T: TryFrom<SerializedBytes, Error = SerializedBytesError> + IndexableEntry + std::fmt::Debug
>(
    from: DateTime<Utc>,
    until: DateTime<Utc>,
    index: String,
    link_tag: Option<LinkTag>,
    strategy: SearchStrategy,
    limit: Option<usize>,
) -> IndexResult<Vec<T>> {
    let order = if from > until {
        Order::Desc
    } else {
        Order::Asc
    };

    Ok(match strategy {
        SearchStrategy::Bfs => {
            let paths = find_paths_for_time_span(from, until, index)?;
            let mut results: Vec<T> = vec![];

            for path in paths {
                let paths = path.typed(LinkTypes::PathLink)?.children_paths()?;
                let mut indexes = paths
                    .clone()
                    .into_iter()
                    .map(|path_child| {
                        let links = get_links(path_child.path_entry_hash()?, LinkTypes::Index, link_tag.clone())?;
                        Ok(links)
                    })
                    .collect::<IndexResult<Vec<Vec<Link>>>>()?
                    .into_iter()
                    .flatten()
                    .map(|link| match get(link.target, GetOptions::latest())? {
                        Some(chunk) => Ok(Some(chunk.entry().to_app_option::<T>()?.ok_or(
                            IndexError::InternalError("Expected element to contain app entry data"),
                        )?)),
                        None => Ok(None),
                    })
                    .filter_map(|val| {
                        if val.is_ok() {
                            let val = val.unwrap();
                            if val.is_some() {
                                Some(Ok(val.unwrap()))
                            } else {
                                None
                            }
                        } else {
                            Some(Err(val.err().unwrap()))
                        }
                    })
                    .collect::<IndexResult<Vec<T>>>()?;
                results.append(&mut indexes);
            }
            match order {
                Order::Desc => {
                    results.sort_by(|a, b| b.entry_time().partial_cmp(&a.entry_time()).unwrap());
                }
                Order::Asc => {
                    results.sort_by(|a, b| a.entry_time().partial_cmp(&b.entry_time()).unwrap());
                }
            }

            results
        }
        SearchStrategy::Dfs => {
            make_dfs_search::<T>(index, &from, &until, &order, limit, link_tag)?
        }
    })
}

/// Takes a timestamp and creates an index path
pub(crate) fn create_for_timestamp(index: String, time: DateTime<Utc>) -> IndexResult<Path> {
    let time_index = get_index_for_timestamp(time);
    let path = time_index.new(index)?;
    Ok(path)
}
