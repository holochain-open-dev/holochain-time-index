//use chrono::{Duration, DurationRound};

use std::{
    convert::{TryFrom, TryInto},
    ops::Sub,
};

use chrono::{NaiveDate, NaiveDateTime};
use hdk::{
    hash_path::path::{Component, Path},
    prelude::{SerializedBytes, UnsafeBytes},
};

use crate::entries::{Index, IndexIndex, TimeIndex, WrappedPath};
use crate::errors::{IndexError, IndexResult};

/// Helper function to get serializedbytes of IndexIndex and make this cleaner in the code
impl IndexIndex {
    pub fn get_sb(self) -> IndexResult<SerializedBytes> {
        Ok(self.try_into()?)
    }
}

/// Helper function to get serializedbytes of TimeIndex and make this cleaner in the code
impl TimeIndex {
    pub fn get_sb(self) -> IndexResult<SerializedBytes> {
        Ok(self.try_into()?)
    }
}

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

impl TryFrom<Component> for IndexIndex {
    type Error = IndexError;

    fn try_from(data: Component) -> Result<Self, Self::Error> {
        let time_index: Vec<u8> = data.into();
        Ok(IndexIndex::try_from(SerializedBytes::from(
            UnsafeBytes::from(time_index),
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
                &IndexIndex::try_from(components[0].clone()).unwrap().0,
            );
            components.remove(0);
        };
        for component in components {
            let time_index = TimeIndex::try_from(component.clone());
            if time_index.is_err() {
                debug_struct.field(
                    "index",
                    &Index::try_from(component)
                        .expect("Could not convert component into TimeIndex or IndexIndex"),
                )
            } else {
                debug_struct.field(
                    "time_index",
                    &time_index
                        .expect("Could not convert component into TimeIndex or IndexIndex")
                        .0,
                )
            };
        }
        debug_struct.finish()
    }
}

impl std::fmt::Debug for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("Index");
        debug_struct.field("from", &self.from.as_secs().to_string());
        debug_struct.field("until", &self.until.as_secs().to_string());
        debug_struct.field("diff", &self.until.sub(self.from));
        debug_struct.field(
            "timestamp",
            &NaiveDateTime::from_timestamp(self.from.as_secs() as i64, self.from.subsec_nanos()),
        );
        debug_struct.field(
            "timestamp_until",
            &NaiveDateTime::from_timestamp(self.until.as_secs() as i64, self.until.subsec_nanos()),
        );
        debug_struct.finish()
    }
}

/// Convert a path into a NaiveDateTime; will fill datetime from path elements and will default to value 1 if no path component
/// is found for a given datetime element
impl TryInto<NaiveDateTime> for WrappedPath {
    type Error = IndexError;

    fn try_into(self) -> Result<NaiveDateTime, Self::Error> {
        let data = self.0;
        let path_comps: Vec<Component> = data.into();
        Ok(NaiveDate::from_ymd(
            TimeIndex::try_from(
                path_comps
                    .get(1)
                    .ok_or(IndexError::InternalError(
                        "Expected at least one elements to convert to DateTime",
                    ))?
                    .to_owned(),
            )?
            .0 as i32,
            TimeIndex::try_from(
                path_comps
                    .get(2)
                    .unwrap_or(&Component::from(
                        SerializedBytes::try_from(TimeIndex(1))?.bytes().to_owned(),
                    ))
                    .to_owned(),
            )?
            .0,
            TimeIndex::try_from(
                path_comps
                    .get(3)
                    .unwrap_or(&Component::from(
                        SerializedBytes::try_from(TimeIndex(1))?.bytes().to_owned(),
                    ))
                    .to_owned(),
            )?
            .0,
        )
        .and_hms(
            TimeIndex::try_from(
                path_comps
                    .get(4)
                    .unwrap_or(&Component::from(
                        SerializedBytes::try_from(TimeIndex(1))?.bytes().to_owned(),
                    ))
                    .to_owned(),
            )?
            .0,
            TimeIndex::try_from(
                path_comps
                    .get(5)
                    .unwrap_or(&Component::from(
                        SerializedBytes::try_from(TimeIndex(1))?.bytes().to_owned(),
                    ))
                    .to_owned(),
            )?
            .0,
            TimeIndex::try_from(
                path_comps
                    .get(6)
                    .unwrap_or(&Component::from(
                        SerializedBytes::try_from(TimeIndex(1))?.bytes().to_owned(),
                    ))
                    .to_owned(),
            )?
            .0,
        ))
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
