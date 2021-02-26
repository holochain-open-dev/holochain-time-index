use chrono::{DateTime, Datelike, NaiveDateTime, Timelike, Utc};
use hdk3::{hash_path::path::Component, prelude::*};

use crate::{
    DayIndex, HourIndex, MinuteIndex, MonthIndex, SecondIndex, TimeIndex, YearIndex,
    TIME_INDEX_DEPTH,
};

pub fn add_time_index_to_path<T: TryInto<SerializedBytes, Error = SerializedBytesError> + From<u32>>(
    time_path: &mut Vec<Component>,
    from_timestamp: &DateTime<Utc>,
    time_index: TimeIndex,
) -> HdkResult<()> {
    let from_time = match time_index {
        TimeIndex::Year => from_timestamp.year() as u32,
        TimeIndex::Month => from_timestamp.month(),
        TimeIndex::Day => from_timestamp.day(),
        TimeIndex::Hour => {
            if TIME_INDEX_DEPTH.contains(&time_index) {
                from_timestamp.hour()
            } else {
                return Ok(());
            }
        }
        TimeIndex::Minute => {
            if TIME_INDEX_DEPTH.contains(&time_index) {
                from_timestamp.minute()
            } else {
                return Ok(());
            }
        }
        TimeIndex::Second => {
            if TIME_INDEX_DEPTH.contains(&time_index) {
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

pub fn get_time_path(
    from: std::time::Duration
) -> HdkResult<Vec<Component>> {
    //Create timestamp "tree"; i.e 2020 -> 02 -> 16 -> chunk
    //Create from timestamp
    let from_timestamp = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(from.as_secs_f64() as i64, from.subsec_nanos()),
        Utc,
    );
    let mut time_path = vec![];
    add_time_index_to_path::<YearIndex>(&mut time_path, &from_timestamp, TimeIndex::Year)?;
    add_time_index_to_path::<MonthIndex>(&mut time_path, &from_timestamp, TimeIndex::Month)?;
    add_time_index_to_path::<DayIndex>(&mut time_path, &from_timestamp, TimeIndex::Day)?;
    add_time_index_to_path::<HourIndex>(&mut time_path, &from_timestamp, TimeIndex::Hour)?;
    add_time_index_to_path::<MinuteIndex>(&mut time_path, &from_timestamp, TimeIndex::Minute)?;
    add_time_index_to_path::<SecondIndex>(&mut time_path, &from_timestamp, TimeIndex::Second)?;

    Ok(time_path)
}
