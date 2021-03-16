use std::time::Duration;

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Timelike, Utc};
use hdk3::{hash_path::path::Component, prelude::*};

use crate::entries::{Index, IndexIndex, IndexType, TimeIndex, WrappedPath};
use crate::errors::{IndexError, IndexResult};
use crate::{MAX_CHUNK_INTERVAL, TIME_INDEX_DEPTH};

pub(crate) fn get_path_links_on_path(path: &Path) -> IndexResult<Vec<Path>> {
    let links = path
        .children()?
        .into_inner()
        .into_iter()
        .map(|link| Ok(Path::try_from(&link.tag)?))
        .collect::<IndexResult<Vec<Path>>>()?;
    Ok(links)
}

pub(crate) fn find_paths_for_time_span(
    from: DateTime<Utc>,
    until: DateTime<Utc>,
    index: String,
) -> IndexResult<Vec<Path>> {
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

    Ok(paths)
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
            let time_index_depth = unwrap_time_index_depth();
            if time_index_depth.contains(&time_index) {
                ()
            } else {
                return Ok(path);
            }
        }
        IndexType::Minute => {
            let time_index_depth = unwrap_time_index_depth();
            if time_index_depth.contains(&time_index) {
                ()
            } else {
                return Ok(path);
            }
        }
        IndexType::Second => {
            let time_index_depth = unwrap_time_index_depth();
            if time_index_depth.contains(&time_index) {
                ()
            } else {
                return Ok(path);
            }
        }
    };
    debug!("Finding links on IndexType: {:#?}\n\n", time_index);
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

pub(crate) fn add_time_index_to_path<
    T: TryInto<SerializedBytes, Error = SerializedBytesError> + From<u32>,
>(
    time_path: &mut Vec<Component>,
    from_timestamp: &DateTime<Utc>,
    time_index: IndexType,
) -> IndexResult<()> {
    let from_time = match time_index {
        IndexType::Year => from_timestamp.year() as u32,
        IndexType::Month => from_timestamp.month(),
        IndexType::Day => from_timestamp.day(),
        IndexType::Hour => {
            let time_index_depth = unwrap_time_index_depth();
            if time_index_depth.contains(&time_index) {
                from_timestamp.hour()
            } else {
                return Ok(());
            }
        }
        IndexType::Minute => {
            let time_index_depth = unwrap_time_index_depth();
            if time_index_depth.contains(&time_index) {
                from_timestamp.minute()
            } else {
                return Ok(());
            }
        }
        IndexType::Second => {
            let time_index_depth = unwrap_time_index_depth();
            if time_index_depth.contains(&time_index) {
                from_timestamp.second()
            } else {
                return Ok(());
            }
        }
    };
    time_path.push(Component::from(
        T::try_into(T::from(from_time))?.bytes().to_owned(),
    ));
    Ok(())
}

pub(crate) fn get_time_path(
    index: String,
    from: std::time::Duration,
) -> IndexResult<Vec<Component>> {
    //Create timestamp "tree"; i.e 2020 -> 02 -> 16 -> chunk
    //Create from timestamp
    let from_timestamp = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(from.as_secs_f64() as i64, from.subsec_nanos()),
        Utc,
    );
    let mut time_path = vec![Component::from(
        IndexIndex(index).get_sb()?.bytes().to_owned(),
    )];
    add_time_index_to_path::<TimeIndex>(&mut time_path, &from_timestamp, IndexType::Year)?;
    add_time_index_to_path::<TimeIndex>(&mut time_path, &from_timestamp, IndexType::Month)?;
    add_time_index_to_path::<TimeIndex>(&mut time_path, &from_timestamp, IndexType::Day)?;
    add_time_index_to_path::<TimeIndex>(&mut time_path, &from_timestamp, IndexType::Hour)?;
    add_time_index_to_path::<TimeIndex>(&mut time_path, &from_timestamp, IndexType::Minute)?;
    add_time_index_to_path::<TimeIndex>(&mut time_path, &from_timestamp, IndexType::Second)?;

    Ok(time_path)
}

pub(crate) fn get_index_for_timestamp(time: DateTime<Utc>) -> Index {
    let now = std::time::Duration::new(time.timestamp() as u64, time.timestamp_subsec_nanos());
    let time_frame = unwrap_chunk_interval_lock();

    let chunk_index_start = (now.as_nanos() as f64 / time_frame.as_nanos() as f64).floor() as u64;
    let chunk_start = time_frame.as_nanos() as u64 * chunk_index_start;
    let chunk_end = time_frame.as_nanos() as u64 * (chunk_index_start + 1_u64);

    let chunk_start = std::time::Duration::from_nanos(chunk_start);
    let chunk_end = std::time::Duration::from_nanos(chunk_end);
    Index {
        from: chunk_start,
        until: chunk_end,
    }
}

fn get_next_level_path(
    paths: Vec<Path>,
    from: &DateTime<Utc>,
    until: &DateTime<Utc>,
    time_index: IndexType,
) -> IndexResult<Vec<Path>> {
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
            let time_index_depth = unwrap_time_index_depth();
            if time_index_depth.contains(&time_index) {
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
            let time_index_depth = unwrap_time_index_depth();
            if time_index_depth.contains(&time_index) {
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
            let time_index_depth = unwrap_time_index_depth();
            if time_index_depth.contains(&time_index) {
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

    let mut out = vec![];
    for path in paths {
        let mut lower_paths: Vec<Path> = path
            .children()?
            .into_inner()
            .into_iter()
            .map(|link| Ok(Path::try_from(&link.tag)?))
            .filter_map(|path| {
                debug!("got path in iter: {:#?}", path);
                if path.is_ok() {
                    let path = path.unwrap();
                    let path_wrapped = WrappedPath(path.clone());
                    let chrono_path: IndexResult<NaiveDateTime> = path_wrapped.try_into();
                    if chrono_path.is_err() {
                        return Some(Err(chrono_path.err().unwrap()));
                    };
                    let chrono_path = chrono_path.unwrap();
                    debug!(
                        "Chrono path: {:?}, from time: {:?}, until time: {:?}",
                        chrono_path, from_time, until_time
                    );
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

pub(crate) fn unwrap_chunk_interval_lock() -> Duration {
    *MAX_CHUNK_INTERVAL
        .read()
        .expect("Could not read from MAX_CHUNK_INTERVAL")
}

pub(crate) fn unwrap_time_index_depth() -> Vec<IndexType> {
    TIME_INDEX_DEPTH
        .read()
        .expect("Could not read from TIME_INDEX_DEPTH")
        .clone()
}

mod util_tests {
    #[test]
    fn test_get_chunk_time() {
        use crate::utils::get_index_for_timestamp;

        //Hard coded interval
        let interval = 10;
        let chunk = get_index_for_timestamp(chrono::Utc::now());
        assert_eq!(chunk.from.as_secs() % interval, 0);
        assert_eq!(chunk.until.as_secs() % interval, 0);
    }

    #[test]
    fn translate_sort() {
        let str_nums = vec!["2", "1"];
        let nums = str_nums
            .clone()
            .into_iter()
            .map(|val| val.parse::<i32>().unwrap())
            .collect::<Vec<i32>>();

        let permutation = permutation::sort(&nums[..]);
        let ordered_nums = permutation.apply_slice(&str_nums[..]);
        assert_eq!(ordered_nums, vec!["1", "2"]);
    }
}
