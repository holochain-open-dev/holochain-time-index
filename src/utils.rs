use std::time::Duration;

use chrono::{DateTime, Datelike, NaiveDateTime, Timelike, Utc};
use hdk3::{hash_path::path::Component, prelude::*};

use crate::entries::{
    DayIndex, HourIndex, IndexIndex, MinuteIndex, MonthIndex, SecondIndex, TimeIndex,
    TimeIndexType, YearIndex,
};
use crate::{MAX_CHUNK_INTERVAL, TIME_INDEX_DEPTH};

pub(crate) fn get_path_links_on_path(path: &Path) -> ExternResult<Vec<Path>> {
    let links = path
        .children()?
        .into_inner()
        .into_iter()
        .map(|link| get(link.target, GetOptions::content()))
        .collect::<ExternResult<Vec<Option<Element>>>>()?
        .into_iter()
        .filter(|link| link.is_some())
        .map(|val| {
            let val = val.unwrap();
            let val: Path = val
                .entry()
                .to_app_option()?
                .ok_or(WasmError::Zome(String::from(
                    "Could not deserialize link target into time Path",
                )))?;
            Ok(val)
        })
        .collect::<ExternResult<Vec<Path>>>()?;
    Ok(links)
}

/// Tries to find the newest time period one level down from current path position
/// Returns path passed in params if maximum depth has been reached
pub(crate) fn find_newest_time_path<
    T: TryFrom<SerializedBytes, Error = SerializedBytesError> + Into<u32>,
>(
    path: Path,
    time_index: TimeIndexType,
) -> ExternResult<Path> {
    match time_index {
        TimeIndexType::Year => (),
        TimeIndexType::Month => (),
        TimeIndexType::Day => (),
        TimeIndexType::Hour => {
            let time_index_depth = unwrap_time_index_depth();
            if time_index_depth.contains(&time_index) {
                ()
            } else {
                return Ok(path);
            }
        }
        TimeIndexType::Minute => {
            let time_index_depth = unwrap_time_index_depth();
            if time_index_depth.contains(&time_index) {
                ()
            } else {
                return Ok(path);
            }
        }
        TimeIndexType::Second => {
            let time_index_depth = unwrap_time_index_depth();
            if time_index_depth.contains(&time_index) {
                ()
            } else {
                return Ok(path);
            }
        }
    };
    debug!("Finding links on TimeIndexType: {:#?}\n\n", time_index);
    //Pretty sure this filter and sort logic can be faster; first rough pass to get basic pieces in place
    let mut links = get_path_links_on_path(&path)?;
    if links.len() == 0 {
        return Err(WasmError::Zome(format!(
            "Could not find any time paths for path: {:?}",
            path
        )));
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
    time_index: TimeIndexType,
) -> ExternResult<()> {
    let from_time = match time_index {
        TimeIndexType::Year => from_timestamp.year() as u32,
        TimeIndexType::Month => from_timestamp.month(),
        TimeIndexType::Day => from_timestamp.day(),
        TimeIndexType::Hour => {
            let time_index_depth = unwrap_time_index_depth();
            if time_index_depth.contains(&time_index) {
                from_timestamp.hour()
            } else {
                return Ok(());
            }
        }
        TimeIndexType::Minute => {
            let time_index_depth = unwrap_time_index_depth();
            if time_index_depth.contains(&time_index) {
                from_timestamp.minute()
            } else {
                return Ok(());
            }
        }
        TimeIndexType::Second => {
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
) -> ExternResult<Vec<Component>> {
    //Create timestamp "tree"; i.e 2020 -> 02 -> 16 -> chunk
    //Create from timestamp
    let from_timestamp = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(from.as_secs_f64() as i64, from.subsec_nanos()),
        Utc,
    );
    let mut time_path = vec![Component::from(
        IndexIndex(index).get_sb()?.bytes().to_owned(),
    )];
    add_time_index_to_path::<YearIndex>(&mut time_path, &from_timestamp, TimeIndexType::Year)?;
    add_time_index_to_path::<MonthIndex>(&mut time_path, &from_timestamp, TimeIndexType::Month)?;
    add_time_index_to_path::<DayIndex>(&mut time_path, &from_timestamp, TimeIndexType::Day)?;
    add_time_index_to_path::<HourIndex>(&mut time_path, &from_timestamp, TimeIndexType::Hour)?;
    add_time_index_to_path::<MinuteIndex>(&mut time_path, &from_timestamp, TimeIndexType::Minute)?;
    add_time_index_to_path::<SecondIndex>(&mut time_path, &from_timestamp, TimeIndexType::Second)?;

    Ok(time_path)
}

pub(crate) fn get_chunk_for_timestamp(time: DateTime<Utc>) -> TimeIndex {
    debug!("get chunk for ts");
    let now = std::time::Duration::new(time.timestamp() as u64, time.timestamp_subsec_nanos());
    debug!("Now: {:#?}", now);
    let time_frame = unwrap_chunk_interval_lock();
    debug!("got time frame: {:#?}", time_frame);
    let chunk_index_start = (now.as_nanos() as f64 / time_frame.as_nanos() as f64).floor() as u64;
    let chunk_start = time_frame.as_nanos() as u64 * chunk_index_start;
    let chunk_end = time_frame.as_nanos() as u64 * (chunk_index_start + 1_u64);

    let chunk_start = std::time::Duration::from_nanos(chunk_start);
    let chunk_end = std::time::Duration::from_nanos(chunk_end);
    TimeIndex {
        from: chunk_start,
        until: chunk_end,
    }
}

pub(crate) fn unwrap_chunk_interval_lock() -> Duration {
    *MAX_CHUNK_INTERVAL
        .read()
        .expect("Could not read from MAX_CHUNK_INTERVAL")
}

pub(crate) fn unwrap_time_index_depth() -> Vec<TimeIndexType> {
    TIME_INDEX_DEPTH
        .read()
        .expect("Could not read from TIME_INDEX_DEPTH")
        .clone()
}

// pub(crate) fn unwrap_spam_limit() -> usize {
//     *ENFORCE_SPAM_LIMIT
//         .read()
//         .expect("Could not read from ENFORCE_SPAM_LIMIT")
// }

mod util_tests {
    #[test]
    fn test_get_chunk_time() {
        use crate::utils::get_chunk_for_timestamp;

        //Hard coded interval
        let interval = 10;
        let chunk = get_chunk_for_timestamp(chrono::Utc::now());
        assert_eq!(chunk.from.as_secs() % interval, 0);
        assert_eq!(chunk.until.as_secs() % interval, 0);
    }
}
