//! This DHT aims to be one solution (of many) to the DHT hostpotting problem that can occur in holochain DHT's when many links are made from one entry.
//! This hotspotting occurs as the original author (and their surrounding hash neighbourhood?) of an entry is responsible for storing and resolving
//! all links from the given authored entry. As a result if a given entry becomes very popular then it can be left up to one or a few nodes
//! to handle all traffic flowing through this part of the DHT.
//!
//!
//! DNA functioning
//!
//! The main component that allows the mitigation of DHT hotspots is 1) the time delimited chunks and 2) the agent focused validation that occurs on each chunk. 
//! For any given chunk an agent cannot make more than DIRECT_CHUNK_LINK_LIMIT direct links on a given chunk. Once this limit has been met, subsequent 
//! links must be linked together in a linked list shape. Here the target entry of the last direct link they created is the source entry of the linked list. 
//! An agent can make links like this until their total links reaches the ENFORCE_SPAM_LIMIT limit at which point no further links are allowed. 
//! The first limit is a measure to protect DHT hotspots in a busy DHT with a high MAX_CHUNK_INTERVAL & the second limit is supposed to block clear/obvious spam.
//! 
//! This DNA's variables is expected to be static. That means its expected that the: DIRECT_CHUNK_LINK_LIMIT, ENFORCE_SPAM_LIMIT & MAX_CHUNK_INTERVAL will 
//! stay the same throughout the lifetime of the DHT. This is done to make validation possible in situations where DHT could occur. 
//! If limits are able to change; we have no way to reliably know if an agent is operating on old limits by consequence of being out of touch 
//! with latest DHT state or if the agent is malicious and pretending they do not see the new limits.
//! If you can guarantee that fragmentation of the DHT will not happen then its possible that limit updates could work. 
//! We may also add support for the reduction of limits in the future; but increasing limits will not land.
//! 
//! For now if one wishes to increase these values its recommended to create a new DNA/DHT and link to it from the current. 
//!
//! This DNA exposes a few helper functions to make operating with chunked data easy. Ones of note are: 
//! ['TimeChunk::get_current_chunk()'], ['TimeChunk::get_latest_chunk()'], ['TimeChunk::get_chunks_for_time_span()'], ['TimeChunk::add_link()'] & ['TimeChunk::get_links()']
//! 
//! ['TimeChunk::get_current_chunk()'] will take the current time as denoted by sys_time() and return null or a chunk that can be used to served entries for the current time.
//! ['TimeChunk::get_latest_chunk()'] will search though the DNA's time "index" and find the last commited chunk and return it.
//! ['TimeChunk::get_chunks_for_time_span()'] will return all chunks that served in a given time span
//! ['TimeChunk::add_link()'] will create a link on a chunk. This will happen either directly or by the linked list fashion as explained above.
//! ['TimeChunk::get_links()'] will get links from the chunk, recursing down any linked lists to ensure that all links are returned for a given chunk.

#[macro_use]
extern crate lazy_static;

use std::time::Duration;

use hdk::hash_path::anchor::Anchor;
use hdk::prelude::*;

mod impls;
mod methods;
mod utils;

#[hdk_entry(id = "time_chunk", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Clone)]
pub struct TimeChunk {
    pub from: std::time::Duration,
    pub until: std::time::Duration,
}

#[hdk_entry(id = "year_index", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Eq, PartialEq)]
pub struct YearIndex(u32);
#[hdk_entry(id = "month_index", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Clone)]
pub struct MonthIndex(u32);
#[hdk_entry(id = "day_index", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Clone)]
pub struct DayIndex(u32);
#[hdk_entry(id = "hour_index", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Clone)]
pub struct HourIndex(u32);
#[hdk_entry(id = "minute_index", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Clone)]
pub struct MinuteIndex(u32);
#[hdk_entry(id = "second_index", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Clone)]
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

entry_defs![
    YearIndex::entry_def(),
    MonthIndex::entry_def(),
    DayIndex::entry_def(),
    HourIndex::entry_def(),
    MinuteIndex::entry_def(),
    SecondIndex::entry_def(),
    Anchor::entry_def()
];

// Extern zome functions

#[hdk_extern]
fn init(_: ()) -> ExternResult<InitCallbackResult> {
    debug!("Init agent zome fn\n\n\n\n\n");
    //NOTE: Only for testing
    methods::create_genesis_chunk()?;
    Ok(InitCallbackResult::Pass)
}

#[hdk_extern]
fn validate(_data: ValidateData) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
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

#[hdk_extern]
fn get_genesis_chunk(_: ()) -> ExternResult<OptionTimeChunk> {
    debug!("Getting genesis chunk");
    Ok(OptionTimeChunk(methods::get_genesis_chunk()?))
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
