use std::time::Duration;

use hdk3::prelude::*;

#[derive(Clone, SerializedBytes, Debug, Serialize, Deserialize, PartialEq, PartialOrd, Ord, Eq)]
pub struct Index {
    pub from: Duration,
    pub until: Duration,
}

#[derive(Clone, SerializedBytes, Debug, Serialize, Deserialize)]
pub struct IndexIndex(pub String);

#[derive(Clone, Eq, PartialEq, SerializedBytes, Debug, Serialize, Deserialize)]
pub struct YearIndex(pub u32);

#[derive(Clone, SerializedBytes, Debug, Serialize, Deserialize)]
pub struct MonthIndex(pub u32);

#[derive(Clone, SerializedBytes, Debug, Serialize, Deserialize)]
pub struct DayIndex(pub u32);

#[derive(Clone, SerializedBytes, Debug, Serialize, Deserialize)]
pub struct HourIndex(pub u32);

#[derive(Clone, SerializedBytes, Debug, Serialize, Deserialize)]
pub struct MinuteIndex(pub u32);

#[derive(Clone, SerializedBytes, Debug, Serialize, Deserialize)]
pub struct SecondIndex(pub u32);

/// Wrapper around hdk path that allows us to make our own impls
pub struct WrappedPath(pub Path);

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum IndexType {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}
