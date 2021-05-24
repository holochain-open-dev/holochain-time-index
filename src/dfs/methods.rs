use chrono::{DateTime, NaiveDateTime, Utc};
use hdk::prelude::*;

use crate::entries::{IndexType, WrappedPath};
use crate::errors::IndexResult;
use crate::Order;
use crate::search::get_naivedatetime;

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

    let chosen_path = paths.pop().unwrap();
    //debug!("Using path: {:#?}", WrappedPath(chosen_path.clone()));

    //Iterate over paths and get children for each and only return paths where path is between from & until naivedatetime
    let mut out = vec![];
    let mut lower_paths: Vec<Path> = chosen_path
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
