use chrono::{DateTime, NaiveDateTime, Utc};
use hdk::{hash_path::path::Component, prelude::*};

use crate::entries::{IndexType, StringIndex, WrappedPath};
use crate::errors::IndexResult;
use crate::search::get_naivedatetime;
use crate::utils::find_divergent_time;

/// Find all paths which exist between from & until timestamps with starting index
/// This function is executed in BFS maner and will return all paths between from/until bounds
pub(crate) fn find_paths_for_time_span<PLT: Clone>(
    from: DateTime<Utc>,
    until: DateTime<Utc>,
    index: String,
    path_link_type: PLT
) -> IndexResult<Vec<Path>> 
    where ScopedLinkType: TryFrom<PLT, Error = WasmError> {
    //Start path with index
    let mut paths = vec![Component::from(
        StringIndex(index).get_sb()?.bytes().to_owned(),
    )];
    //Determine and create the starting path based on index and divergence between timestamps
    let (mut found_path, index_level) = find_divergent_time(&from, &until)?;
    paths.append(&mut found_path);
    let mut paths = vec![Path::from(paths)];
    // debug!(
    //     "Path before query starts: {:#?} starting with: {:?}",
    //     paths
    //         .clone()
    //         .into_iter()
    //         .map(|path| WrappedPath(path))
    //         .collect::<Vec<WrappedPath>>(),
    //     index_level
    // );

    for level in index_level {
        paths = get_next_level_path_bfs(paths, &from, &until, &level, path_link_type.clone())?;
        // debug!(
        //     "Now have paths: {:#?} at level: {:#?}",
        //     paths
        //         .clone()
        //         .into_iter()
        //         .map(|path| WrappedPath(path))
        //         .collect::<Vec<WrappedPath>>(),
        //     level
        // );
    }

    Ok(paths)
}

/// For a given index type get the naivedatetime representation of from & until and use to compare against path components
/// found as children to supplied path. Will only return paths where path timeframe is inbetween from & until.
/// This function is executed in bfs maner and is exhastive in that it will get all children for each path and
/// will append each child path to the resulting vec
pub(crate) fn get_next_level_path_bfs<PLT: Clone>(
    paths: Vec<Path>,
    from: &DateTime<Utc>,
    until: &DateTime<Utc>,
    index_type: &IndexType,
    path_link_type: PLT
) -> IndexResult<Vec<Path>> 
    where ScopedLinkType: TryFrom<PLT, Error = WasmError> {
    //Get the naivedatetime representation for from & until
    let (from_time, until_time) = match get_naivedatetime(from, until, index_type) {
        Some(tuple) => tuple,
        None => return Ok(paths),
    };

    //Iterate over paths and get children for each and only return paths where path is between from & until naivedatetime
    let mut out = vec![];
    for path in paths {
        let mut lower_paths: Vec<Path> = path.typed(path_link_type.clone())?
            .children_paths()?
            .into_iter()
            .filter_map(|path| {
                let path_wrapped = WrappedPath(path.path.clone());
                let chrono_path: IndexResult<NaiveDateTime> = path_wrapped.try_into();
                if chrono_path.is_err() {
                    return Some(Err(chrono_path.err().unwrap()));
                };
                let chrono_path = chrono_path.unwrap();
                if chrono_path >= from_time && chrono_path <= until_time {
                    Some(Ok(path.path))
                } else {
                    None
                }
            })
            .collect::<IndexResult<Vec<Path>>>()?;    
        out.append(&mut lower_paths);
    }
    Ok(out)
}
