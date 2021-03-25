use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Timelike, Utc};
use hdk::{hash_path::path::Component, prelude::*};

use crate::entries::{IndexIndex, IndexType, WrappedPath};
use crate::errors::{IndexError, IndexResult};
use crate::utils::{find_divergent_time, get_path_links_on_path};
use crate::TIME_INDEX_DEPTH;

/// Find all paths which exist between from & until timestamps with starting index
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
    //debug!("Path before query starts: {:#?} starting with: {:?}", paths, index_level);

    for level in index_level {
        paths = get_next_level_path(paths, &from, &until, level)?;
    }

    Ok(paths)
}

/// For a given index type get the naivedatetime representation of from & until and use to compare against path components
/// found as children to supplied path. Will only return paths where path timeframe is inbetween from & until.
fn get_next_level_path(
    paths: Vec<Path>,
    from: &DateTime<Utc>,
    until: &DateTime<Utc>,
    time_index: IndexType,
) -> IndexResult<Vec<Path>> {
    //Get the naivedatetime representation for from & until
    let (from_time, until_time) = match time_index {
        IndexType::Year => (
            NaiveDate::from_ymd(from.year(), 1, 1).and_hms(1, 1, 1),
            NaiveDate::from_ymd(until.year(), 1, 1).and_hms(1, 1, 1),
        ),
        IndexType::Month => (
            NaiveDate::from_ymd(from.year(), from.month(), 1).and_hms(1, 1, 1),
            NaiveDate::from_ymd(until.year(), until.month(), 1).and_hms(1, 1, 1),
        ),
        IndexType::Day => (
            NaiveDate::from_ymd(from.year(), from.month(), from.day()).and_hms(1, 1, 1),
            NaiveDate::from_ymd(until.year(), until.month(), until.day()).and_hms(1, 1, 1),
        ),
        IndexType::Hour => {
            if TIME_INDEX_DEPTH.contains(&time_index) {
                (
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
                )
            } else {
                return Ok(paths);
            }
        }
        IndexType::Minute => {
            if TIME_INDEX_DEPTH.contains(&time_index) {
                (
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
                )
            } else {
                return Ok(paths);
            }
        }
        IndexType::Second => {
            if TIME_INDEX_DEPTH.contains(&time_index) {
                (
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
                )
            } else {
                return Ok(paths);
            }
        }
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
            if TIME_INDEX_DEPTH.contains(&time_index) {
                ()
            } else {
                return Ok(path);
            }
        }
        IndexType::Minute => {
            if TIME_INDEX_DEPTH.contains(&time_index) {
                ()
            } else {
                return Ok(path);
            }
        }
        IndexType::Second => {
            if TIME_INDEX_DEPTH.contains(&time_index) {
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
