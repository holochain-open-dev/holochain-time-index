use chrono::{DateTime, Datelike, NaiveDateTime, Timelike, Utc};
use hdk::{hash_path::path::Component, prelude::*};
//use hdi::prelude::Timestamp;

use crate::entries::{Index, IndexType, StringIndex, TimeIndex};
use crate::errors::{IndexResult};
use crate::{INDEX_DEPTH, MAX_CHUNK_INTERVAL};

/// Find the overlapping path between two times and return vec of queries at given IndexTypes which still need to be performed
pub(crate) fn find_divergent_time(
    from: &DateTime<Utc>,
    until: &DateTime<Utc>,
) -> IndexResult<(Vec<Component>, Vec<IndexType>)> {
    //Make year comparison
    let mut path = if from.year() == until.year() {
        vec![Component::from(
            TimeIndex(from.year() as u32).get_sb()?.bytes().to_owned(),
        )]
    } else {
        return Ok((
            vec![],
            vec![
                IndexType::Year,
                IndexType::Month,
                IndexType::Day,
                IndexType::Hour,
                IndexType::Minute,
                IndexType::Second,
            ],
        ));
    };
    //Make month comparison
    if from.month() == until.month() {
        path.push(Component::from(
            TimeIndex(from.month() as u32).get_sb()?.bytes().to_owned(),
        ));
    } else {
        return Ok((
            path,
            vec![
                IndexType::Month,
                IndexType::Day,
                IndexType::Hour,
                IndexType::Minute,
                IndexType::Second,
            ],
        ));
    };
    //Make day comparison
    if from.day() == until.day() {
        path.push(Component::from(
            TimeIndex(from.day() as u32).get_sb()?.bytes().to_owned(),
        ));
    } else {
        return Ok((
            path,
            vec![
                IndexType::Day,
                IndexType::Hour,
                IndexType::Minute,
                IndexType::Second,
            ],
        ));
    };
    //Check if index depth is allowed and make hour comparison
    if INDEX_DEPTH.contains(&IndexType::Hour) {
        if from.hour() == until.hour() {
            path.push(Component::from(
                TimeIndex(from.hour() as u32).get_sb()?.bytes().to_owned(),
            ));
        } else {
            return Ok((
                path,
                vec![IndexType::Hour, IndexType::Minute, IndexType::Second],
            ));
        }
    } else {
        return Ok((
            path,
            vec![IndexType::Hour, IndexType::Minute, IndexType::Second],
        ));
    };
    //Check if index depth is allowed and make minute comparison
    if INDEX_DEPTH.contains(&IndexType::Minute) {
        if from.minute() == until.minute() {
            path.push(Component::from(
                TimeIndex(from.minute() as u32).get_sb()?.bytes().to_owned(),
            ));
        } else {
            return Ok((path, vec![IndexType::Minute, IndexType::Second]));
        }
    } else {
        return Ok((path, vec![IndexType::Minute, IndexType::Second]));
    };
    //Check if index depth is allowed and make second comparison
    if INDEX_DEPTH.contains(&IndexType::Second) {
        if from.second() == until.second() {
            path.push(Component::from(
                TimeIndex(from.second() as u32).get_sb()?.bytes().to_owned(),
            ));
        } else {
            return Ok((path, vec![IndexType::Second]));
        }
    } else {
        return Ok((path, vec![IndexType::Second]));
    };
    return Ok((path, vec![]))
}

/// Create a timestamp path tree from a given duration and index
pub(crate) fn get_time_path(
    index: String,
    from: std::time::Duration,
) -> IndexResult<Vec<Component>> {
    let from_timestamp = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp_opt(from.as_secs_f64() as i64, from.subsec_nanos()).unwrap(),
        Utc,
    );
    let mut time_path = vec![Component::from(
        StringIndex(index).get_sb()?.bytes().to_owned(),
    )];
    add_time_index_to_path::<TimeIndex>(&mut time_path, &from_timestamp, IndexType::Year)?;
    add_time_index_to_path::<TimeIndex>(&mut time_path, &from_timestamp, IndexType::Month)?;
    add_time_index_to_path::<TimeIndex>(&mut time_path, &from_timestamp, IndexType::Day)?;
    add_time_index_to_path::<TimeIndex>(&mut time_path, &from_timestamp, IndexType::Hour)?;
    add_time_index_to_path::<TimeIndex>(&mut time_path, &from_timestamp, IndexType::Minute)?;
    add_time_index_to_path::<TimeIndex>(&mut time_path, &from_timestamp, IndexType::Second)?;
    // debug!("Indexing with path lenght: {:#?}", time_path.len());

    Ok(time_path)
}

/// Add TimeIndex component to time path whilst checking if time component depth is allowed as determined by libs configuration vars
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
            if INDEX_DEPTH.contains(&time_index) {
                from_timestamp.hour()
            } else {
                return Ok(());
            }
        }
        IndexType::Minute => {
            if INDEX_DEPTH.contains(&time_index) {
                from_timestamp.minute()
            } else {
                return Ok(());
            }
        }
        IndexType::Second => {
            if INDEX_DEPTH.contains(&time_index) {
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

/// Determine correct chunk position for a given timestamp
pub(crate) fn get_index_for_timestamp(time: DateTime<Utc>) -> Index {
    let now = std::time::Duration::new(time.timestamp() as u64, time.timestamp_subsec_nanos());
    let time_frame = MAX_CHUNK_INTERVAL.as_nanos() as f64;

    let chunk_index_start = (now.as_nanos() as f64 / time_frame).floor() as u64;
    let chunk_start = time_frame as u64 * chunk_index_start;
    let chunk_end = time_frame as u64 * (chunk_index_start + 1_u64);

    let chunk_start = std::time::Duration::from_nanos(chunk_start);
    let chunk_end = std::time::Duration::from_nanos(chunk_end);
    Index {
        from: chunk_start,
        until: chunk_end,
    }
}

// pub fn get_now() -> IndexResult<Timestamp> {
//     Ok(sys_time()?)
// }

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
