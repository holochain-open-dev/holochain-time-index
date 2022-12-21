use chrono::{DateTime, NaiveDateTime, Utc};
use hdk::hash_path::path::Component;
use hdk::prelude::info::ScopedLinkType;
use hdk::prelude::*;
use petgraph::graph::NodeIndex;
use petgraph::visit::Dfs;
use std::fmt::Debug;

use crate::dfs::SearchState;
use crate::entries::{Index, IndexType, StringIndex, WrappedPath};
use crate::errors::{IndexError, IndexResult};
use crate::search::get_naivedatetime;
use crate::utils::find_divergent_time;
use crate::{IndexableEntry, Order, DEFAULT_INDEX_DEPTH, INDEX_DEPTH};

pub(crate) fn make_dfs_search<
    T: TryFrom<SerializedBytes, Error = SerializedBytesError> + IndexableEntry + Debug,
    ILT: LinkTypeFilterExt + Clone,
    PLT: Clone,
>(
    index: String,
    from: &DateTime<Utc>,
    until: &DateTime<Utc>,
    order: &Order,
    limit: Option<usize>,
    link_tag: Option<LinkTag>,
    index_link_type: ILT,
    path_link_type: PLT,
) -> IndexResult<Vec<T>>
where
    ScopedLinkType: TryFrom<PLT, Error = WasmError>,
{
    let mut out: Vec<T> = vec![];
    let mut search_state = SearchState::new();
    //Start path with index
    let mut paths = vec![Component::from(
        StringIndex(index).get_sb()?.bytes().to_owned(),
    )];
    //Determine and create the starting path based on index and divergence between timestamps
    let (mut found_path, index_level) = find_divergent_time(&from, &until)?;
    paths.append(&mut found_path);
    let mut paths = vec![Path::from(paths)];
    // debug!(
    //     "Path before dfs query starts: {:#?} index levels: {:?}",
    //     paths
    //         .clone()
    //         .into_iter()
    //         .map(|path| WrappedPath(path.clone()))
    //         .collect::<Vec<WrappedPath>>(),
    //     index_level
    // );

    //Populate our search state Graph with found paths
    search_state.populate_from_paths(&paths, 0)?;

    //Iterate over remaining search levels to get next paths in DFS maner
    //There will only ever be one path here since we are getting the common root path for from/until
    let components: Vec<Component> = paths[0].clone().into();
    let mut search_node = NodeIndex::new(components.len() - 1);
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
        paths =
            get_next_level_path_dfs(paths, &from, &until, &level, &order, path_link_type.clone())?;
        //If we dont get any paths at the next index level then we should return empty vec
        if paths.len() == 0 {
            return Ok(vec![]);
        }
        // debug!(
        //     "Now have paths: {:#?} at level: {:#?}",
        //     paths
        //         .clone()
        //         .into_iter()
        //         .map(|path| WrappedPath(path))
        //         .collect::<Vec<WrappedPath>>(),
        //     level
        // );
        // search_state.display_dot_repr();

        //Save the retreived paths to the Graph for later use
        //Search node returned so we can add the next path links from the first path item in previous recursion
        search_node =
            search_state.populate_from_paths_forward(paths.clone(), depth, search_node)?;
    }

    // search_state.display_dot_repr();

    //Determine how far down the graph we should search before trying to get final links/entries
    let max_depth_size = DEFAULT_INDEX_DEPTH.len() + INDEX_DEPTH.len();
    let break_at_limit = limit.is_some();
    //Start dfs search
    let mut dfs = Dfs::new(&search_state.0, NodeIndex::from(0));
    let mut end_node = None;
    let mut has_searched = vec![];

    loop {
        let next_node = dfs.next(&search_state.0);
        // debug!(
        //     "Got next node: {:#?}",
        //     next_node.map(|node| search_state.0.node_weight(node).unwrap())
        // );
        if next_node.is_none() {
            break;
        };
        let node = search_state.0.node_weight(next_node.unwrap()).unwrap();
        //Check if at bottom of index graph, if so then get links/entries
        if node.0.len() == max_depth_size {
            // debug!("Found node with correct depth, getting index links");
            end_node = next_node;
            let mut indexes = Path::from(
                search_state
                    .0
                    .node_weight(end_node.unwrap())
                    .unwrap()
                    .0
                    .clone(),
            )
            .typed(path_link_type.clone())?
            .children_paths()?;
            indexes.sort_by(|a, b| {
                let index_chunk = Index::try_from(a.path.clone()).unwrap();
                let index_chunk_b = Index::try_from(b.path.clone()).unwrap();
                match order {
                    Order::Desc => index_chunk_b.from.partial_cmp(&index_chunk.from).unwrap(),
                    Order::Asc => index_chunk.from.partial_cmp(&index_chunk_b.from).unwrap(),
                }
            });
            for index in indexes {
                // debug!(
                //     "Getting links for path: {:#?}",
                //     WrappedPath(index.clone())
                // );
                let mut links = get_links(
                    index.path_entry_hash()?,
                    index_link_type.clone(),
                    link_tag.clone(),
                )?
                .into_iter()
                .map(|link| {
                    match get(
                        link.target
                            .into_entry_hash()
                            .expect("Could not get entry hash"),
                        GetOptions::latest(),
                    )? {
                        Some(chunk) => Ok(Some(chunk.entry().to_app_option::<T>()?.ok_or(
                            IndexError::InternalError("Expected element to contain app entry data"),
                        )?)),
                        None => Ok(None),
                    }
                })
                .filter_map(|val| {
                    if val.is_ok() {
                        let val = val.unwrap();
                        if val.is_some() {
                            let val = val.unwrap();
                            match order {
                                Order::Desc => {
                                    if val.entry_time() <= *from && val.entry_time() >= *until {
                                        Some(Ok(val))
                                    } else {
                                        None
                                    }
                                }
                                Order::Asc => {
                                    if val.entry_time() >= *from && val.entry_time() <= *until {
                                        Some(Ok(val))
                                    } else {
                                        None
                                    }
                                }
                            }
                        } else {
                            None
                        }
                    } else {
                        Some(Err(val.err().unwrap()))
                    }
                })
                .collect::<IndexResult<Vec<T>>>()?;
                out.append(&mut links);
                if break_at_limit {
                    if out.len() >= limit.unwrap() {
                        break;
                    }
                }
            }
            if break_at_limit {
                if out.len() >= limit.unwrap() {
                    break;
                }
            }
        } else if end_node.is_some() {
            if !has_searched.contains(&next_node.unwrap()) {
                //Not at the bottom of the tree/graph but should be at the next lowest point of index, here we will grab then next set of indexes
                let node = Path::from(
                    search_state
                        .0
                        .node_weight(next_node.unwrap())
                        .unwrap()
                        .0
                        .clone(),
                );
                let node_components: Vec<Component> = node.clone().into();
                let index_type = match node_components.len() {
                    1 => IndexType::Year,
                    2 => IndexType::Month,
                    3 => IndexType::Day,
                    4 => IndexType::Hour,
                    5 => IndexType::Minute,
                    6 => IndexType::Second,
                    _ => return Err(IndexError::InternalError("Expected path to be length 2-7")),
                };
                //debug!("No node found with correct depth but node found where last end_node was of correct depth, executing next branch of search. Has index: {:#?}", next_node.unwrap());
                paths = get_next_level_path_dfs(
                    vec![node],
                    &from,
                    &until,
                    &index_type,
                    &order,
                    path_link_type.clone(),
                )?;
                // debug!(
                //     "Got next paths in dfs search tree: {:#?}",
                //     paths
                //         .clone()
                //         .into_iter()
                //         .map(|path| WrappedPath(path.clone()))
                //         .collect::<Vec<WrappedPath>>()
                // );

                //Add the founds paths as indexes on the current node item
                search_state
                    .populate_next_nodes_from_position(paths.clone(), next_node.unwrap())?;

                //Clone the current stack to keep a list of already visited nodes
                let mut visited_stack = dfs.stack.clone();

                //Start a new search with new graph & old state
                //Dfs::move_to(&mut dfs, next_node.unwrap());
                dfs = Dfs::new(&search_state.0, next_node.unwrap());
                dfs.stack.append(&mut visited_stack);
                dfs.stack.dedup();

                //Keep array of all end nodes which were visited but required further DHT calls as to avoid infinite recursion when accessing this node again on next loop iteration
                has_searched.push(next_node.unwrap());
            }
        }
    }

    // search_state.display_dot_repr();

    Ok(if break_at_limit {
        match order {
            Order::Desc => out.sort_by(|a, b| b.entry_time().partial_cmp(&a.entry_time()).unwrap()),
            Order::Asc => out.sort_by(|a, b| a.entry_time().partial_cmp(&b.entry_time()).unwrap()),
        }
        if out.len() > limit.unwrap() {
            let _vec2 = out.split_off(limit.unwrap());
            out
        } else {
            out
        }
    } else {
        out
    })
}

/// For a given index type get the naivedatetime representation of from & until and use to compare against path components
/// found as children to supplied path. Will only return paths where path timeframe is inbetween from & until. This function
/// is executed in a dfs maner and will choose one path (dependant on order; highest (Order::Desc) or lowest value (Order::Asc))
/// And then get the next set of paths from the choosen path
pub(crate) fn get_next_level_path_dfs<PLT: Clone>(
    mut paths: Vec<Path>,
    from: &DateTime<Utc>,
    until: &DateTime<Utc>,
    index_type: &IndexType,
    order: &Order,
    path_link_type: PLT,
) -> IndexResult<Vec<Path>>
where
    ScopedLinkType: TryFrom<PLT, Error = WasmError>,
{
    //Get the naivedatetime representation for from & until
    let (from_time, until_time) = match get_naivedatetime(from, until, index_type) {
        Some(tuple) => tuple,
        None => return Ok(paths),
    };

    paths.sort_by(|patha, pathb| {
        let chrono_path_a: NaiveDateTime = WrappedPath(patha.clone()).try_into().unwrap();
        let chrono_path_b: NaiveDateTime = WrappedPath(pathb.clone()).try_into().unwrap();
        match order {
            Order::Desc => chrono_path_a.partial_cmp(&chrono_path_b).unwrap(),
            Order::Asc => chrono_path_b.partial_cmp(&chrono_path_a).unwrap(),
        }
    });

    let chosen_path = paths.pop().unwrap();
    // debug!("Got chosen path: {:#?}", WrappedPath(chosen_path.clone()));

    //Iterate over paths and get children for each and only return paths where path is between from & until naivedatetime
    let mut lower_paths: Vec<Path> = chosen_path
        .typed(path_link_type)?
        .children_paths()?
        .into_iter()
        .filter_map(|path| {
            // debug!("Got path in map {:#?}", path);
            let path_wrapped = WrappedPath(path.path.clone());
            let chrono_path: IndexResult<NaiveDateTime> = path_wrapped.clone().try_into();
            // debug!("Got path in lowerpaths fn: {:#?}. {:#?}. {:#?}/{:#?}. {:#?}", path_wrapped, chrono_path, from_time, until_time, index_type);
            if chrono_path.is_err() {
                return Some(Err(chrono_path.err().unwrap()));
            };
            let chrono_path = chrono_path.unwrap();
            match order {
                Order::Desc => {
                    if chrono_path <= from_time && chrono_path >= until_time {
                        Some(Ok(path.path))
                    } else {
                        None
                    }
                }
                Order::Asc => {
                    if chrono_path >= from_time && chrono_path <= until_time {
                        Some(Ok(path.path))
                    } else {
                        None
                    }
                }
            }
        })
        .collect::<IndexResult<Vec<Path>>>()?;
    lower_paths.sort_by(|a, b| {
        let path_wrapped = WrappedPath(a.clone());
        let path_wrapped_b = WrappedPath(b.clone());
        let chrono_path_a: IndexResult<NaiveDateTime> = path_wrapped.try_into();
        let chrono_path_b: IndexResult<NaiveDateTime> = path_wrapped_b.try_into();
        match order {
            Order::Desc => chrono_path_b
                .unwrap()
                .partial_cmp(&chrono_path_a.unwrap())
                .unwrap(),
            Order::Asc => chrono_path_a
                .unwrap()
                .partial_cmp(&chrono_path_b.unwrap())
                .unwrap(),
        }
    });
    Ok(lower_paths)
}
