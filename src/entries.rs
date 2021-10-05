use chrono::NaiveDateTime;
use std::time::Duration;

use hdk::prelude::*;

#[derive(Clone, SerializedBytes, Serialize, Deserialize, PartialEq, PartialOrd, Ord, Eq)]
pub struct Index {
    pub from: Duration,
    pub until: Duration,
}

#[derive(Clone, SerializedBytes, Debug, Serialize, Deserialize)]
pub struct StringIndex(pub String);

#[derive(Clone, Eq, PartialEq, SerializedBytes, Debug, Serialize, Deserialize)]
pub struct TimeIndex(pub u32);

/// Wrapper around hdk path that allows us to make our own impls
#[derive(Clone)]
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

pub type IndexSegment = (String, Option<NaiveDateTime>, Option<Index>);
