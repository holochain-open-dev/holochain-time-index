use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, Utc};
use hdk3::{hash_path::path::Component, prelude::*};

use crate::errors::{IndexError, IndexResult};
use crate::utils::{
    add_time_index_to_path, find_newest_time_path, find_paths_for_time_span,
    get_index_for_timestamp, get_time_path, unwrap_chunk_interval_lock,
};
use crate::EntryChunkIndex;
use crate::{
    entries::{Index, IndexIndex, IndexType, TimeIndex},
    IndexableEntry,
};

impl Index {
    /// Create a new time index
    pub(crate) fn new(&self, index: String) -> IndexResult<Path> {
        //These validations are to help zome callers; but should also be present in validation rules
        if self.from > sys_time()? {
            return Err(IndexError::RequestError(
                "Time index cannot start in the future",
            ));
        };
        let max_chunk_interval = unwrap_chunk_interval_lock();
        if self.until - self.from != max_chunk_interval {
            return Err(IndexError::RequestError(
                "Time index should use period equal to max interval set by DNA",
            ));
        };
        if self.from.as_millis() % max_chunk_interval.as_millis() != 0 {
            return Err(IndexError::RequestError(
                "Time index does not follow index interval ordering",
            ));
        };

        let mut time_path = get_time_path(index, self.from)?;
        time_path.push(SerializedBytes::try_from(self)?.bytes().to_owned().into());

        //Create time tree
        let time_path = Path::from(time_path);
        time_path.ensure()?;
        Ok(time_path)
    }
}

/// Get current index using sys_time as source for time
pub fn get_current_index(index: String) -> IndexResult<Option<Path>> {
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
        .collect::<IndexResult<Vec<Duration>>>()?;
    let permutation = permutation::sort_by(&ser_path[..], |a, b| a.partial_cmp(&b).unwrap());
    let mut ordered_indexes = permutation.apply_slice(&indexes[..]);
    ordered_indexes.reverse();

    match ordered_indexes.pop() {
        Some(link) => match get(link.target, GetOptions::content())? {
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
        .collect::<IndexResult<Vec<Duration>>>()?;
    let permutation = permutation::sort_by(&ser_path[..], |a, b| a.partial_cmp(&b).unwrap());
    let mut ordered_indexes: Vec<Link> = permutation.apply_slice(&indexes[..]);
    ordered_indexes.reverse();

    match ordered_indexes.pop() {
        Some(link) => match get(link.target, GetOptions::content())? {
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
            .collect::<IndexResult<Vec<EntryChunkIndex>>>()?;
        out.append(&mut indexes);
    }
    out.sort_by(|a, b| a.index.from.partial_cmp(&b.index.from).unwrap());
    out.reverse();

    Ok(out)
}

/// Get all links that exist for some time period between from -> until
pub(crate) fn get_links_for_time_span(
    from: DateTime<Utc>,
    until: DateTime<Utc>,
    index: String,
    link_tag: Option<LinkTag>,
) -> IndexResult<Vec<Link>> {
    let paths = find_paths_for_time_span(from, until, index)?;
    //debug!("Got paths after search: {:#?}", paths);
    let mut out: Vec<Link> = vec![];

    for path in paths {
        let paths = path.children()?.into_inner();
        let mut indexes = paths
            .clone()
            .into_iter()
            .map(|link| {
                let path = Path::try_from(&link.tag)?;
                let links = get_links(path.hash()?, link_tag.clone())?.into_inner();
                Ok(links)
            })
            .collect::<IndexResult<Vec<Vec<Link>>>>()?
            .into_iter()
            .flatten()
            .collect();
        out.append(&mut indexes);
    }
    out.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap());
    out.reverse();

    Ok(out)
}

/// Get all links that exist for some time period between from -> until
pub(crate) fn get_links_and_load_for_time_span<
    T: TryFrom<SerializedBytes, Error = SerializedBytesError> + IndexableEntry,
>(
    from: DateTime<Utc>,
    until: DateTime<Utc>,
    index: String,
    link_tag: Option<LinkTag>,
) -> IndexResult<Vec<T>> {
    let paths = find_paths_for_time_span(from, until, index)?;
    let mut out: Vec<T> = vec![];

    for path in paths {
        let paths = path.children()?.into_inner();
        let mut indexes = paths
            .clone()
            .into_iter()
            .map(|link| {
                let path = Path::try_from(&link.tag)?;
                let links = get_links(path.hash()?, link_tag.clone())?.into_inner();
                Ok(links)
            })
            .collect::<IndexResult<Vec<Vec<Link>>>>()?
            .into_iter()
            .flatten()
            .map(|link| match get(link.target, GetOptions::content())? {
                Some(chunk) => {
                    Ok(chunk
                        .entry()
                        .to_app_option()?
                        .ok_or(IndexError::InternalError(
                            "Expected element to contain app entry data",
                        ))?)
                }
                None => Err(IndexError::InternalError(
                    "Expected link target to contain point to an entry",
                )),
            })
            .collect::<IndexResult<Vec<T>>>()?;
        out.append(&mut indexes);
    }
    out.sort_by(|a, b| a.entry_time().partial_cmp(&b.entry_time()).unwrap());
    out.reverse();

    Ok(out)
}

/// Takes a timestamp and creates an index path
pub(crate) fn create_for_timestamp(index: String, time: DateTime<Utc>) -> IndexResult<Path> {
    let time_index = get_index_for_timestamp(time);
    let path = time_index.new(index)?;
    Ok(path)
}
