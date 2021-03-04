use chrono::{DateTime, Datelike, NaiveDateTime, Timelike, Utc};
use hdk3::{hash_path::path::Component, prelude::*};

use crate::{
    DayIndex, HourIndex, MinuteIndex, MonthIndex, SecondIndex, TimeIndex, YearIndex,
    TIME_INDEX_DEPTH,
};

pub fn get_path_links_on_path(path: &Path) -> ExternResult<Vec<Path>> {
    let links = get_links(path.hash()?, None)?
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
pub fn find_newest_time_path<
    T: TryFrom<SerializedBytes, Error = SerializedBytesError> + Into<u32>,
>(
    path: Path,
    time_index: TimeIndex,
) -> ExternResult<Path> {
    match time_index {
        TimeIndex::Year => (),
        TimeIndex::Month => (),
        TimeIndex::Day => (),
        TimeIndex::Hour => {
            if TIME_INDEX_DEPTH.contains(&time_index) {
                ()
            } else {
                return Ok(path);
            }
        }
        TimeIndex::Minute => {
            if TIME_INDEX_DEPTH.contains(&time_index) {
                ()
            } else {
                return Ok(path);
            }
        }
        TimeIndex::Second => {
            if TIME_INDEX_DEPTH.contains(&time_index) {
                ()
            } else {
                return Ok(path);
            }
        }
    };
    debug!("Finding links on TimeIndex: {:#?}\n\n", time_index);
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

pub fn add_time_index_to_path<
    T: TryInto<SerializedBytes, Error = SerializedBytesError> + From<u32>,
>(
    time_path: &mut Vec<Component>,
    from_timestamp: &DateTime<Utc>,
    time_index: TimeIndex,
) -> ExternResult<()> {
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

pub fn get_time_path(from: std::time::Duration) -> ExternResult<Vec<Component>> {
    //Create timestamp "tree"; i.e 2020 -> 02 -> 16 -> chunk
    //Create from timestamp
    let from_timestamp = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(from.as_secs_f64() as i64, from.subsec_nanos()),
        Utc,
    );
    let mut time_path = vec![Component::from("time")];
    add_time_index_to_path::<YearIndex>(&mut time_path, &from_timestamp, TimeIndex::Year)?;
    add_time_index_to_path::<MonthIndex>(&mut time_path, &from_timestamp, TimeIndex::Month)?;
    add_time_index_to_path::<DayIndex>(&mut time_path, &from_timestamp, TimeIndex::Day)?;
    add_time_index_to_path::<HourIndex>(&mut time_path, &from_timestamp, TimeIndex::Hour)?;
    add_time_index_to_path::<MinuteIndex>(&mut time_path, &from_timestamp, TimeIndex::Minute)?;
    add_time_index_to_path::<SecondIndex>(&mut time_path, &from_timestamp, TimeIndex::Second)?;

    Ok(time_path)
}
