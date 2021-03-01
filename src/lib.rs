//! This DHT aims to be one solution (of many) to the DHT hostpotting problem that can occur in holochain DHT's when many links are made from one entry.
//! This hotspotting occurs as the original author (and their surrounding hash neighbourhood?) of an entry is responsible for storing and resolving
//! all links from the given authored entry. As a result if a given entry becomes very popular then it can be left up to one or a few nodes
//! to handle all traffic flowing through this part of the DHT.
//!
//!
//! DNA functioning
//!

#[macro_use]
extern crate lazy_static;

use std::time::Duration;

use hdk3::hash_path::anchor::Anchor;
use hdk3::prelude::*;

mod impls;
mod methods;
mod utils;

#[hdk_entry(id = "time_chunk", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone)]
pub struct TimeChunk {
    pub from: std::time::Duration,
    pub until: std::time::Duration,
}

#[hdk_entry(id = "year_index", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct YearIndex(u32);
#[hdk_entry(id = "month_index", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone)]
pub struct MonthIndex(u32);
#[hdk_entry(id = "day_index", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone)]
pub struct DayIndex(u32);
#[hdk_entry(id = "hour_index", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone)]
pub struct HourIndex(u32);
#[hdk_entry(id = "minute_index", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone)]
pub struct MinuteIndex(u32);
#[hdk_entry(id = "second_index", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone)]
pub struct SecondIndex(u32);

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TimeIndex {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}

#[hdk_extern]
fn entry_defs(_: ()) -> ExternResult<EntryDefsCallbackResult> {
    Ok(vec![
        YearIndex::entry_def(),
        MonthIndex::entry_def(),
        DayIndex::entry_def(),
        HourIndex::entry_def(),
        MinuteIndex::entry_def(),
        SecondIndex::entry_def(),
        Anchor::entry_def(),
    ]
    .into())
}

// Extern zome functions

#[hdk_extern]
fn init(_: ()) -> ExternResult<InitCallbackResult> {
    //NOTE: Only for testing
    methods::create_genesis_chunk()?;
    Ok(InitCallbackResult::Pass)
}

#[hdk_extern]
fn create_chunk(chunk: TimeChunk) -> ExternResult<TimeChunk> {
    chunk.create_chunk(false)?;
    Ok(chunk)
}

#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct GetPreviousChunkRequest {
    pub chunk: TimeChunk,
    pub hops: u32
}

#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct OptionTimeChunk(Option<TimeChunk>);

#[hdk_extern]
fn get_previous_chunk(data: GetPreviousChunkRequest) -> ExternResult<OptionTimeChunk> {
    let chunk = data.chunk.get_previous_chunk(data.hops)?;
    Ok(OptionTimeChunk(chunk))
}

#[hdk_extern]
fn get_current_chunk(_: ()) -> ExternResult<OptionTimeChunk> {
    let chunk = TimeChunk::get_current_chunk()?;
    Ok(OptionTimeChunk(chunk))
}

#[hdk_extern]
fn get_latest_chunk(_: ()) -> ExternResult<TimeChunk> {
    let chunk = TimeChunk::get_latest_chunk()?;
    Ok(chunk)
}

// Configuration
// TODO this needs to be derived from DNA's properties

lazy_static! {
    //Set the membrane list for this DNA
    pub static ref MEMBRANE: Option<Vec<AgentInfo>> = None;

    //Point at which links coming from a given agent need to be added together as linked list vs a standard link on given chunk
    pub static ref DIRECT_CHUNK_LINK_LIMIT: usize = 20;
    //Point at which links are considered spam and linked list expressions are not allowed
    pub static ref ENFORCE_SPAM_LIMIT: usize = 50;
    //Max duration of given time chunk
    pub static ref MAX_CHUNK_INTERVAL: std::time::Duration = std::time::Duration::new(300, 0);
    //Determine what depth of time index chunks should be hung from; this is the only piece that can be left as so
    //and not directly derived from DNA properties
    pub static ref TIME_INDEX_DEPTH: Vec<TimeIndex> = {
        if *MAX_CHUNK_INTERVAL < Duration::from_secs(1) {
            vec![TimeIndex::Second, TimeIndex::Minute, TimeIndex::Hour, TimeIndex::Day]
        } else if *MAX_CHUNK_INTERVAL < Duration::from_secs(60) {
            vec![TimeIndex::Minute, TimeIndex::Hour, TimeIndex::Day]
        } else if *MAX_CHUNK_INTERVAL < Duration::from_secs(3600) {
            vec![TimeIndex::Hour, TimeIndex::Day]
        } else {
            vec![TimeIndex::Day]
        }
    };
    //Useful for testing
    pub static ref ENABLE_VALIDATION: bool = false;
}
