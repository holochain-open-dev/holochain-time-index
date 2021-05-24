use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, Utc};
use hdk::{hash_path::path::Component, prelude::*};
use petgraph::graph::NodeIndex;
use petgraph::visit::Dfs;

use crate::dfs::SearchState;
use crate::search::{find_newest_time_path, find_paths_for_time_span, get_next_level_path_dfs};
use crate::utils::{
    add_time_index_to_path, find_divergent_time, get_index_for_timestamp, get_time_path,
};
use crate::{
    entries::{Index, IndexIndex, IndexType, TimeIndex, WrappedPath},
    EntryChunkIndex, IndexableEntry, SearchStrategy, DEFAULT_INDEX_DEPTH, INDEX_DEPTH,
    MAX_CHUNK_INTERVAL,
};
use crate::{
    errors::{IndexError, IndexResult},
    Order,
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
        Some(link) => match get(link.target, GetOptions::latest())? {
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

    //TODO: dont error out if cant find link target; just use next link
    match ordered_indexes.pop() {
        Some(link) => match get(link.target, GetOptions::latest())? {
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
    index: String,
    from: DateTime<Utc>,
    until: DateTime<Utc>,
    link_tag: Option<LinkTag>,
    strategy: SearchStrategy,
    limit: Option<usize>,
) -> IndexResult<Vec<Link>> {
    debug!("Getting links for time span");
    let order = if from > until {
        Order::Desc
    } else {
        Order::Asc
    };

    Ok(match strategy {
        SearchStrategy::Bfs => {
            if limit.is_some() {
                debug!("hc_time_index::get_links_for_time_span: WARNING: Limit not supported on Bfs strategy. All links between bounds will be retrieved and returned");
            };
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
            //out.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap());
            //out.reverse();
            out
        }
        SearchStrategy::Dfs => {
            let mut out: Vec<Link> = vec![];
            let mut search_state = SearchState::new();
            //Start path with index
            let mut paths = vec![Component::from(
                IndexIndex(index).get_sb()?.bytes().to_owned(),
            )];
            //Determine and create the starting path based on index and divergence between timestamps
            let (mut found_path, index_level) = find_divergent_time(from, until)?;
            paths.append(&mut found_path);
            let mut paths = vec![Path::from(paths)];
            debug!(
                "Path before dfs query starts: {:#?} index levels: {:?}",
                paths
                    .clone()
                    .into_iter()
                    .map(|path| WrappedPath(path.clone()))
                    .collect::<Vec<WrappedPath>>(),
                index_level
            );

            //Populate our search state Graph with found paths
            search_state.populate_from_paths(paths.clone(), 0)?;

            //Iterate over remaining search levels to get next paths in DFS maner
            //There will only ever be one path here since we are getting the common root path for from/until
            let components: Vec<Component> = paths[0].clone().into();
            let mut search_node = NodeIndex::new(components.len() -1);
            for level in index_level {
                let depth = match level {
                    IndexType::Year => 1,
                    IndexType::Month => 2,
                    IndexType::Day => 3,
                    IndexType::Hour => 4,
                    IndexType::Minute => 5,
                    IndexType::Second => 6,
                };
                //Get the next paths for the current path
                paths = get_next_level_path_dfs(paths, &from, &until, &level, &order)?;
                debug!(
                    "Now have paths: {:#?} at level: {:#?}",
                    paths
                        .clone()
                        .into_iter()
                        .map(|path| WrappedPath(path))
                        .collect::<Vec<WrappedPath>>(),
                    level
                );
                //Save the retreived paths to the Graph for later use
                //Search node returned so we can add the next path links from the first path item in previous recursion
                search_node = search_state.populate_from_paths_forward(paths.clone(), depth, search_node)?;
            }
            search_state.display_dot_repr();

            //Determine how far down the graph we should search before trying to get final links/entries
            let max_depth_size = DEFAULT_INDEX_DEPTH.len() + INDEX_DEPTH.len();
            let break_at_limit = limit.is_some();
            //Start dfs search
            let mut dfs = Dfs::new(&search_state.0, NodeIndex::from(0));
            let mut end_node = None;

            loop {
                let next_node = dfs.next(&search_state.0);
                debug!("Got next node: {:#?}", next_node.map(|node| search_state.0.node_weight(node).unwrap()));
                if next_node.is_none() {
                    break;
                };
                let node = search_state.0.node_weight(next_node.unwrap()).unwrap();
                //Check if at bottom of index graph, if so then get links/entries
                if node.0.len() == max_depth_size {
                    debug!("Found node with correct depth, getting links");
                    end_node = next_node;
                    let indexes = Path::from(
                        search_state
                            .0
                            .node_weight(end_node.unwrap())
                            .unwrap()
                            .0
                            .clone(),
                    )
                    .children()?
                    .into_inner()
                    .into_iter()
                    .map(|link| Ok(Path::try_from(&link.tag)?))
                    .collect::<IndexResult<Vec<Path>>>()?;
                    for index in indexes {
                        let mut links =
                            get_links(index.hash()?, link_tag.clone())?.into_inner();
                        out.append(&mut links);
                        if break_at_limit {
                            if out.len() > limit.unwrap() {
                                break;
                            }
                        }
                    }
                    if break_at_limit {
                        if out.len() > limit.unwrap() {
                            break;
                        }
                    }
                } else if end_node.is_some() {
                    //Not at the bottom of the tree/graph but should be at the next lowest point of index, here we will grab then next set of indexes
                    let node = Path::from(search_state.0.node_weight(next_node.unwrap()).unwrap().0.clone());
                    let node_components: Vec<Component> = node.clone().into();
                    let index_type = match node_components.len() {
                        2 => IndexType::Year,
                        3 => IndexType::Month,
                        4 => IndexType::Day,
                        5 => IndexType::Hour,
                        6 => IndexType::Minute,
                        7 => IndexType::Second,
                        _ => return Err(IndexError::InternalError("Expected path to be length 2-7"))
                    };
                    debug!("No node found with correct depth but node found where last end_node was of correct depth, executing next branch of search. Has index: {:#?}", next_node.unwrap());
                    paths = get_next_level_path_dfs(vec![node], &from, &until, &index_type, &order)?;
                    debug!("Got next paths: {:#?}", paths
                        .clone()
                        .into_iter()
                        .map(|path| WrappedPath(path.clone()))
                        .collect::<Vec<WrappedPath>>());
                    //Add the founds paths as indexes on the current node item
                    let mut added_indexes = search_state.populate_next_nodes_from_position(paths.clone(), next_node.unwrap())?;
                    //Clone the current stack to keep a list of already visited nodes
                    let mut stack = dfs.stack.clone();
                    //Start a new search with new graph & old state
                    dfs = Dfs::new(&search_state.0, NodeIndex::new(next_node.unwrap().index()));
                    stack.append(&mut added_indexes);
                    dfs.stack = stack;
                }
            }

            search_state.display_dot_repr();

            if break_at_limit {
                if out.len() > limit.unwrap() {
                    out[0..limit.unwrap()].to_owned()
                } else {
                    out
                }
            } else {
                out
            }
        }
    })
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
        out.append(&mut indexes);
    }
    //out.sort_by(|a, b| a.entry_time().partial_cmp(&b.entry_time()).unwrap());
    //out.reverse();

    Ok(out)
}

/// Takes a timestamp and creates an index path
pub(crate) fn create_for_timestamp(index: String, time: DateTime<Utc>) -> IndexResult<Path> {
    let time_index = get_index_for_timestamp(time);
    let path = time_index.new(index)?;
    Ok(path)
}
