use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Timelike, Utc};
use hdk::{hash_path::path::Component, prelude::*};

use crate::entries::IndexType;
use crate::errors::{IndexError, IndexResult};
use crate::INDEX_DEPTH;

pub(crate) fn get_naivedatetime(
    from: &DateTime<Utc>,
    until: &DateTime<Utc>,
    index_type: &IndexType,
) -> Option<(NaiveDateTime, NaiveDateTime)> {
    match index_type {
        IndexType::Year => Some((
            NaiveDate::from_ymd_opt(from.year(), 1, 1).unwrap().and_hms_opt(1, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(until.year(), 1, 1).unwrap().and_hms_opt(1, 1, 1).unwrap(),
        )),
        IndexType::Month => Some((
            NaiveDate::from_ymd_opt(from.year(), from.month(), 1).unwrap().and_hms_opt(1, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(until.year(), until.month(), 1).unwrap().and_hms_opt(1, 1, 1).unwrap(),
        )),
        IndexType::Day => Some((
            NaiveDate::from_ymd_opt(from.year(), from.month(), from.day()).unwrap().and_hms_opt(1, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(until.year(), until.month(), until.day()).unwrap().and_hms_opt(1, 1, 1).unwrap(),
        )),
        IndexType::Hour => {
            if INDEX_DEPTH.contains(&index_type) {
                Some((
                    NaiveDate::from_ymd_opt(from.year(), from.month(), from.day()).unwrap().and_hms_opt(
                        from.hour(),
                        1,
                        1,
                    ).unwrap(),
                    NaiveDate::from_ymd_opt(until.year(), until.month(), until.day()).unwrap().and_hms_opt(
                        until.hour(),
                        1,
                        1,
                    ).unwrap(),
                ))
            } else {
                None
            }
        }
        IndexType::Minute => {
            if INDEX_DEPTH.contains(&index_type) {
                Some((
                    NaiveDate::from_ymd_opt(from.year(), from.month(), from.day()).unwrap().and_hms_opt(
                        from.hour(),
                        from.minute(),
                        1,
                    ).unwrap(),
                    NaiveDate::from_ymd_opt(until.year(), until.month(), until.day()).unwrap().and_hms_opt(
                        until.hour(),
                        until.minute(),
                        1,
                    ).unwrap(),
                ))
            } else {
                None
            }
        }
        IndexType::Second => {
            if INDEX_DEPTH.contains(&index_type) {
                Some((
                    NaiveDate::from_ymd_opt(from.year(), from.month(), from.day()).unwrap().and_hms_opt(
                        from.hour(),
                        from.minute(),
                        from.second(),
                    ).unwrap(),
                    NaiveDate::from_ymd_opt(until.year(), until.month(), until.day()).unwrap().and_hms_opt(
                        until.hour(),
                        until.minute(),
                        until.second(),
                    ).unwrap(),
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
    PLT: Into<ScopedLinkType>
>(
    path: Path,
    time_index: IndexType,
    path_link_type: PLT
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
    let mut links = path.typed(path_link_type)?.children_paths()?;
    if links.len() == 0 {
        return Err(IndexError::Wasm(wasm_error!(WasmErrorInner::Host(String::from("Could not find any time paths for path")))));
    };
    links.sort_by(|a, b| {
        let a_val: Vec<Component> = a.path.to_owned().into();
        let b_val: Vec<Component> = b.path.to_owned().into();
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
    Ok(latest.path)
}
