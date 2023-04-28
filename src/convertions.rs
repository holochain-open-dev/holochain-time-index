use std::convert::{TryFrom, TryInto};

use chrono::{NaiveDate, NaiveDateTime};
use hdk::{
    hash_path::path::{Component, Path},
    prelude::{SerializedBytes, UnsafeBytes},
};

use crate::entries::{Index, IndexSegment, IndexType, StringIndex, TimeIndex, WrappedPath};
use crate::errors::{IndexError, IndexResult};
use crate::INDEX_DEPTH;

impl TryFrom<Path> for Index {
    type Error = IndexError;

    fn try_from(data: Path) -> IndexResult<Index> {
        let path_comps: Vec<Component> = data.into();
        let time_index = path_comps
            .last()
            .ok_or(IndexError::InternalError(
                "Cannot get Index from empty path",
            ))?
            .to_owned();
        let time_index: Vec<u8> = time_index.into();
        let time_index = Index::try_from(SerializedBytes::from(UnsafeBytes::from(time_index)))?;
        Ok(time_index)
    }
}

impl TryFrom<Component> for TimeIndex {
    type Error = IndexError;

    fn try_from(data: Component) -> Result<Self, Self::Error> {
        let time_index: Vec<u8> = data.into();
        Ok(TimeIndex::try_from(SerializedBytes::from(
            UnsafeBytes::from(time_index),
        ))?)
    }
}

impl TryFrom<Component> for StringIndex {
    type Error = IndexError;

    fn try_from(data: Component) -> Result<Self, Self::Error> {
        let bytes: Vec<u8> = data.into();
        Ok(StringIndex::try_from(SerializedBytes::from(
            UnsafeBytes::from(bytes),
        ))?)
    }
}

impl TryFrom<&WrappedPath> for StringIndex {
    type Error = IndexError;

    fn try_from(data: &WrappedPath) -> Result<Self, Self::Error> {
        let path = data.0.clone();
        let components: Vec<Component> = path.into();
        let component = components.first().ok_or(IndexError::InternalError(
            "Expected at least one path element",
        ))?;
        let bytes: Vec<u8> = component.to_owned().into();
        Ok(StringIndex::try_from(SerializedBytes::from(
            UnsafeBytes::from(bytes),
        ))?)
    }
}

impl TryFrom<Component> for Index {
    type Error = IndexError;

    fn try_from(data: Component) -> Result<Self, Self::Error> {
        let time_index: Vec<u8> = data.into();
        Ok(Index::try_from(SerializedBytes::from(UnsafeBytes::from(
            time_index,
        )))?)
    }
}

impl std::fmt::Debug for WrappedPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut components: Vec<Component> = self.0.clone().into();
        let mut debug_struct = f.debug_struct("Path");
        if components.len() > 0 {
            debug_struct.field(
                "index",
                &StringIndex::try_from(components[0].clone()).unwrap().0,
            );
            components.remove(0);
        };
        for component in components {
            let time_index = TimeIndex::try_from(component.clone());
            if time_index.is_err() {
                debug_struct.field(
                    "index",
                    &Index::try_from(component)
                        .expect("Could not convert component into TimeIndex or StringIndex"),
                )
            } else {
                debug_struct.field(
                    "time_index",
                    &time_index
                        .expect("Could not convert component into TimeIndex or StringIndex")
                        .0,
                )
            };
        }
        debug_struct.finish()
    }
}

fn get_time_index_from_components_strict(
    components: &Vec<Component>,
    index: usize,
) -> Result<TimeIndex, IndexError> {
    TimeIndex::try_from(
        components
            .get(index)
            .ok_or(IndexError::InternalError(
                "Expected at least two elements to convert to DateTime",
            ))?
            .to_owned(),
    )
}

fn get_time_index_from_components(
    components: &Vec<Component>,
    index: usize,
) -> Result<TimeIndex, IndexError> {
    TimeIndex::try_from(
        components
            .get(index)
            .unwrap_or(&Component::from(
                SerializedBytes::try_from(TimeIndex(1))?.bytes().to_owned(),
            ))
            .to_owned(),
    )
}

/// Convert a path into a NaiveDateTime; will fill datetime from path elements and will default to value 1 if no path component
/// is found for a given datetime element
impl TryInto<NaiveDateTime> for WrappedPath {
    type Error = IndexError;

    fn try_into(self) -> Result<NaiveDateTime, Self::Error> {
        let data = self.0;
        let path_comps: Vec<Component> = data.into();
        let nd = NaiveDate::from_ymd_opt(
            get_time_index_from_components_strict(&path_comps, 1)?.0 as i32,
            get_time_index_from_components(&path_comps, 2)?.0,
            get_time_index_from_components(&path_comps, 3)?.0,
        ).unwrap();
        //Get the path time components that are optionally present
        let hour = if INDEX_DEPTH.contains(&IndexType::Hour) {
            Some(get_time_index_from_components(&path_comps, 4)?.0)
        } else {
            None
        };
        let min = if INDEX_DEPTH.contains(&IndexType::Minute) {
            Some(get_time_index_from_components(&path_comps, 5)?.0)
        } else {
            None
        };
        let second = if INDEX_DEPTH.contains(&IndexType::Second) {
            Some(get_time_index_from_components(&path_comps, 6)?.0)
        } else {
            None
        };
        let ndt = nd.and_hms_opt(
            hour.unwrap_or(1) as u32,
            min.unwrap_or(1) as u32,
            second.unwrap_or(1) as u32,
        ).unwrap();
        Ok(ndt)
    }
}

impl TryFrom<WrappedPath> for IndexSegment {
    type Error = IndexError;

    fn try_from(data: WrappedPath) -> Result<IndexSegment, Self::Error> {
        let string_index = StringIndex::try_from(&data)?;
        let path_data = data.0.clone();
        let path_comps: Vec<Component> = path_data.clone().into();

        if path_comps.len() == 1 {
            return Ok((string_index.0, None, None));
        }

        let time_index: NaiveDateTime = data.try_into()?;
        let index = if let Ok(index_res) = Index::try_from(path_data) {
            Some(index_res)
        } else {
            None
        };
        Ok((string_index.0, Some(time_index), index))
    }
}

impl From<u32> for TimeIndex {
    fn from(data: u32) -> Self {
        TimeIndex(data)
    }
}

impl Into<u32> for TimeIndex {
    fn into(self) -> u32 {
        self.0
    }
}
