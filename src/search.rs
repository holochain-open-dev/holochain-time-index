use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Timelike, Utc};
use hdk::{hash_path::path::Component, prelude::*};

use crate::entries::{IndexIndex, IndexType, WrappedPath};
use crate::errors::{IndexError, IndexResult};
use crate::utils::{find_divergent_time, get_path_links_on_path};
use crate::{Order, INDEX_DEPTH};

/// Find all paths which exist between from & until timestamps with starting index
/// This function is executed in BFS maner and will return all paths between from/until bounds
pub(crate) fn find_paths_for_time_span(
    from: DateTime<Utc>,
    until: DateTime<Utc>,
    index: String,
) -> IndexResult<Vec<Path>> {
    //Start path with index
    let mut paths = vec![Component::from(
        IndexIndex(index).get_sb()?.bytes().to_owned(),
    )];
    //Determine and create the starting path based on index and divergence between timestamps
    let (mut found_path, index_level) = find_divergent_time(from, until)?;
    paths.append(&mut found_path);
    let mut paths = vec![Path::from(paths)];
    debug!(
        "Path before query starts: {:#?} starting with: {:?}",
        paths
            .clone()
            .into_iter()
            .map(|path| WrappedPath(path))
            .collect::<Vec<WrappedPath>>(),
        index_level
    );

    for level in index_level {
        paths = get_next_level_path_bfs(paths, &from, &until, &level)?;
        debug!(
            "Now have paths: {:#?} at level: {:#?}",
            paths
                .clone()
                .into_iter()
                .map(|path| WrappedPath(path))
                .collect::<Vec<WrappedPath>>(),
            level
        );
    }

    Ok(paths)
}

/// For a given index type get the naivedatetime representation of from & until and use to compare against path components
/// found as children to supplied path. Will only return paths where path timeframe is inbetween from & until. This function 
/// is executed in a dfs maner and will choose one path (dependant on order; highest (Order::Desc) or lowest value (Order::Asc))
/// And then get the next set of paths from the choosen path
pub(crate) fn get_next_level_path_dfs(
    mut paths: Vec<Path>,
    from: &DateTime<Utc>,
    until: &DateTime<Utc>,
    index_type: &IndexType,
    order: &Order,
) -> IndexResult<Vec<Path>> {
    //Get the naivedatetime representation for from & until
    let (from_time, until_time) = match get_naivedatetime(from, until, index_type) {
        Some(tuple) => tuple,
        None => return Ok(paths)
    };

    paths.sort_by(|patha, pathb| {
        let chrono_path_a: NaiveDateTime = WrappedPath(patha.clone()).try_into().unwrap();
        let chrono_path_b: NaiveDateTime = WrappedPath(pathb.clone()).try_into().unwrap();
        match order {
            Order::Desc => chrono_path_b.partial_cmp(&chrono_path_a).unwrap(),
            Order::Asc => chrono_path_a.partial_cmp(&chrono_path_b).unwrap(),
        }
    });

    let choosen_path = paths.pop().unwrap();
    debug!("Using path: {:#?}", WrappedPath(choosen_path.clone()));

    //Iterate over paths and get children for each and only return paths where path is between from & until naivedatetime
    let mut out = vec![];
    let mut lower_paths: Vec<Path> = choosen_path
        .children()?
        .into_inner()
        .into_iter()
        .map(|link| Ok(Path::try_from(&link.tag)?))
        .filter_map(|path| {
            if path.is_ok() {
                let path = path.unwrap();
                let path_wrapped = WrappedPath(path.clone());
                let chrono_path: IndexResult<NaiveDateTime> = path_wrapped.try_into();
                if chrono_path.is_err() {
                    return Some(Err(chrono_path.err().unwrap()));
                };
                let chrono_path = chrono_path.unwrap();
                match order {
                    Order::Desc => {
                        if chrono_path <= from_time && chrono_path >= until_time {
                            Some(Ok(path))
                        } else {
                            None
                        }
                    }
                    Order::Asc => {
                        if chrono_path >= from_time && chrono_path <= until_time {
                            Some(Ok(path))
                        } else {
                            None
                        }
                    }
                }
            } else {
                Some(Err(path.err().unwrap()))
            }
        })
        .collect::<IndexResult<Vec<Path>>>()?;
    lower_paths.sort_by(|a, b| {
        let path_wrapped = WrappedPath(a.clone());
        let path_wrapped_b = WrappedPath(b.clone());
        let chrono_path_a: IndexResult<NaiveDateTime> = path_wrapped.try_into();
        let chrono_path_b: IndexResult<NaiveDateTime> = path_wrapped_b.try_into();
        match order {
            Order::Desc => chrono_path_b.unwrap().partial_cmp(&chrono_path_a.unwrap()).unwrap(),
            Order::Asc => chrono_path_a.unwrap().partial_cmp(&chrono_path_b.unwrap()).unwrap(),
        }
    });
    out.append(&mut lower_paths);
    Ok(out)
}

/// For a given index type get the naivedatetime representation of from & until and use to compare against path components
/// found as children to supplied path. Will only return paths where path timeframe is inbetween from & until.
/// This function is executed in bfs maner and is exhastive in that it will get all children for each path and 
/// will append each child path to the resulting vec
pub(crate) fn get_next_level_path_bfs(
    paths: Vec<Path>,
    from: &DateTime<Utc>,
    until: &DateTime<Utc>,
    index_type: &IndexType,
) -> IndexResult<Vec<Path>> {
    //Get the naivedatetime representation for from & until
    let (from_time, until_time) = match get_naivedatetime(from, until, index_type) {
        Some(tuple) => tuple,
        None => return Ok(paths)
    };

    //Iterate over paths and get children for each and only return paths where path is between from & until naivedatetime
    let mut out = vec![];
    for path in paths {
        let mut lower_paths: Vec<Path> = path
            .children()?
            .into_inner()
            .into_iter()
            .map(|link| Ok(Path::try_from(&link.tag)?))
            .filter_map(|path| {
                if path.is_ok() {
                    let path = path.unwrap();
                    let path_wrapped = WrappedPath(path.clone());
                    let chrono_path: IndexResult<NaiveDateTime> = path_wrapped.try_into();
                    if chrono_path.is_err() {
                        return Some(Err(chrono_path.err().unwrap()));
                    };
                    let chrono_path = chrono_path.unwrap();
                    if chrono_path >= from_time && chrono_path <= until_time {
                        Some(Ok(path))
                    } else {
                        None
                    }
                } else {
                    Some(Err(path.err().unwrap()))
                }
            })
            .collect::<IndexResult<Vec<Path>>>()?;
        out.append(&mut lower_paths);
    }
    Ok(out)
}

fn get_naivedatetime(from: &DateTime<Utc>, until: &DateTime<Utc>, index_type: &IndexType) -> Option<(NaiveDateTime, NaiveDateTime)> {
    match index_type {
        IndexType::Year => Some((
            NaiveDate::from_ymd(from.year(), 1, 1).and_hms(1, 1, 1),
            NaiveDate::from_ymd(until.year(), 1, 1).and_hms(1, 1, 1),
        )),
        IndexType::Month => Some((
            NaiveDate::from_ymd(from.year(), from.month(), 1).and_hms(1, 1, 1),
            NaiveDate::from_ymd(until.year(), until.month(), 1).and_hms(1, 1, 1),
        )),
        IndexType::Day => Some((
            NaiveDate::from_ymd(from.year(), from.month(), from.day()).and_hms(1, 1, 1),
            NaiveDate::from_ymd(until.year(), until.month(), until.day()).and_hms(1, 1, 1),
        )),
        IndexType::Hour => {
            if INDEX_DEPTH.contains(&index_type) {
                Some((
                    NaiveDate::from_ymd(from.year(), from.month(), from.day()).and_hms(
                        from.hour(),
                        1,
                        1,
                    ),
                    NaiveDate::from_ymd(until.year(), until.month(), until.day()).and_hms(
                        until.hour(),
                        1,
                        1,
                    ),
                ))
            } else {
                None
            }
        }
        IndexType::Minute => {
            if INDEX_DEPTH.contains(&index_type) {
                Some((
                    NaiveDate::from_ymd(from.year(), from.month(), from.day()).and_hms(
                        from.hour(),
                        from.minute(),
                        1,
                    ),
                    NaiveDate::from_ymd(until.year(), until.month(), until.day()).and_hms(
                        until.hour(),
                        until.minute(),
                        1,
                    ),
                ))
            } else {
                None
            }
        }
        IndexType::Second => {
            if INDEX_DEPTH.contains(&index_type) {
                Some((
                    NaiveDate::from_ymd(from.year(), from.month(), from.day()).and_hms(
                        from.hour(),
                        from.minute(),
                        from.second(),
                    ),
                    NaiveDate::from_ymd(until.year(), until.month(), until.day()).and_hms(
                        until.hour(),
                        until.minute(),
                        until.second(),
                    ),
                ))
            } else {
                None
            }
        }
    }
}

/// Tries to find the newest time period one level down from current path position
/// Returns path passed in params if maximum depth has been reached
pub(crate) fn find_newest_time_path<
    T: TryFrom<SerializedBytes, Error = SerializedBytesError> + Into<u32>,
>(
    path: Path,
    time_index: IndexType,
) -> IndexResult<Path> {
    match time_index {
        IndexType::Year => (),
        IndexType::Month => (),
        IndexType::Day => (),
        IndexType::Hour => {
            if INDEX_DEPTH.contains(&time_index) {
                ()
            } else {
                return Ok(path);
            }
        }
        IndexType::Minute => {
            if INDEX_DEPTH.contains(&time_index) {
                ()
            } else {
                return Ok(path);
            }
        }
        IndexType::Second => {
            if INDEX_DEPTH.contains(&time_index) {
                ()
            } else {
                return Ok(path);
            }
        }
    };
    //debug!("Finding links on IndexType: {:#?}\n\n", time_index);

    //Pretty sure this filter and sort logic can be faster; first rough pass to get basic pieces in place
    let mut links = get_path_links_on_path(&path)?;
    if links.len() == 0 {
        return Err(IndexError::InternalError(
            "Could not find any time paths for path",
        ));
    };
    links.sort_by(|a, b| {
        let a_val: Vec<Component> = a.to_owned().into();
        let b_val: Vec<Component> = b.to_owned().into();
        let a_u32: u32 = T::try_from(SerializedBytes::from(UnsafeBytes::from(
            a_val[1].as_ref().to_owned(),
        )))
        .unwrap()
        .into();
        let b_u32: u32 = T::try_from(SerializedBytes::from(UnsafeBytes::from(
            b_val[1].as_ref().to_owned(),
        )))
        .unwrap()
        .into();
        a_u32.partial_cmp(&b_u32).unwrap()
    });
    let latest = links.pop().unwrap();
    Ok(latest)
}
