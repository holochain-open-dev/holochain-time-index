//! # HC-Time-Chunking
//!
//! ## Purpose
//!
//! This DHT aims to be one solution (of many) to the DHT hostpotting problem that can occur in holochain DHT's when many links are made from one entry.
//! This hotspotting occurs as the original author (and their surrounding hash neighbourhood?) of an entry is responsible for storing and resolving all links from the given authored entry. As a result if a given entry becomes very popular then it can be left up to one or a few nodes to handle all traffic flowing through this part of the DHT.
//!
//! ## Function
//!
//! The main component that allows the mitigation of DHT hotspots are:
//! 1) time delimited chunks.
//! 2) agent centric validation that occurs on each chunk.
//!
//! ### Time Delimited Chunks
//!
//! The way the chunks are implemented creates a sort of "emergent" linked list behaviour. Emergent because they are not actually linked together; but instead all use the same `MAX_CHUNK_LIMIT` for the timespace they can serve. Thus any given chunk are N chunks away from the genesis chunk and fit into some kind of ordered/connected list. This is useful as it allows for us to determine if a given chunks timespace is allowed in validation. Implementing a traditional linked list over chunks is hard due to the difficulty of gathering consensus about what chunk was the last chunk for some given chunk and which one is the next; each agent might have a different view or perspective over the DHT's data and thus might see different past and future chunks. Enforcing that all chunks serve a `MAX_CHUNK_LIMIT` its easy for any agent to derive what chunks may or may not exist.
//!
//! ### Agent Link Validation
//!
//! For any given chunk an **agent** cannot make more than `DIRECT_CHUNK_LINK_LIMIT` direct links on a given chunk. Once this limit has been met, subsequent links must be linked together in a linked list shape.
//! Here the target entry of the last direct link they created is the source entry of the linked list. An agent can make links like this until their total links reaches the `ENFORCE_SPAM_LIMIT` limit at which point no further links are allowed on this chunk.
//!
//! The first limit is a measure to protect DHT hotspots in a busy DHT with a high `MAX_CHUNK_INTERVAL` & the second limit is supposed to block obvious spam.
//!
//! ### DNA Lifecycle
//!
//! This DNA's variables mentioned above are expected to be static. That means its expected that the: `DIRECT_CHUNK_LINK_LIMIT`, `ENFORCE_SPAM_LIMIT` & `MAX_CHUNK_INTERVAL` will stay the same throughout the lifetime of the DHT. This is done to make validation possible in situations where DHT sharding could occur.
//! If limits are able to change; we have no way to reliably know if an agent is operating on old limits by consequence of being out of touch with latest DHT state or if the agent is malicious and pretending they do not see the new limits. You can see this being an especially big problem when you have two areas of the DHT "merging" and the "outdated" area of the DHT having all of its links in-validated by the agents in the more current of the DHT space.
//!
//! Currently if we wish to update limits we will create a new DNA/DHT and link to the new one from the current.
//!
//! If you can guarantee that fragmentation of the DHT will not happen then its possible to implement limit updates. If this is something you wish to do its recommended that you enforce new limits at some given chunk in the future rather than instantly. This allows you to (hopefully) give enough time for other DHT agents to receive new limit information before its enforced.   
//!
//! ### Exposed Functions
//!
//! This DNA exposes a few helper functions to make operating with chunked data easy. Ones of note are:
//! ['TimeIndex::get_current_chunk()'], ['TimeIndex::get_latest_chunk()'], ['TimeIndex::get_chunks_for_time_span()'], ['TimeIndex::add_link()'] & ['TimeIndex::get_links()']
//!
//! ['TimeIndex::get_current_chunk()'] will take the current time as denoted by sys_time() and return null or a chunk that can be used to served entries for the current time.
//! ['TimeIndex::get_latest_chunk()'] will search though the DNA's time "index" and find the last commited chunk and return it.
//! ['TimeIndex::get_chunks_for_time_span()'] will return all chunks that served in a given time span
//! ['TimeIndex::add_link()'] will create a link on a chunk. This will happen either directly or by the linked list fashion as explained above.
//! ['TimeIndex::get_links()'] will get links from the chunk, recursing down any linked lists to ensure that all links are returned for a given chunk.
//!
//! ### hApp Usage
//!
//! Using the above methods its possible to build an application which places an emphasis on time ordered data (such as a group DM or news feed). Or you can use the time ordered nature of the data as a natural pagination for larger queries where you may wish to aggregate data over a given time period and then perform some further computations over it.

#[macro_use]
extern crate lazy_static;

use chrono::{DateTime, NaiveDateTime, Utc};
use std::sync::RwLock;
use std::time::Duration;

use hdk3::prelude::*;

mod impls;
mod utils;
mod validation;

/// Public methods exposed by lib
pub mod methods;

/// All holochain entries used by this crate
pub mod entries;

mod traits;
/// Trait to impl on entries that you want to add to time index
pub use traits::IndexableEntry;

use entries::TimeIndex;

#[derive(Serialize, Deserialize, Debug)]
pub struct EntryChunkIndex {
    pub time_frame: TimeIndex,
    pub links: Links,
}

/// Gets all links with optional tag link_tag since last_seen time with option to limit number of results by limit
/// Note: if last_seen is a long time ago in a popular DHT then its likely this function will take a very long time to run
pub fn get_addresses_since(
    last_seen: DateTime<Utc>,
    _limit: Option<usize>,
    link_tag: Option<LinkTag>,
) -> ExternResult<Vec<EntryChunkIndex>> {
    let now = sys_time()?;
    let now = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(now.as_secs_f64() as i64, now.subsec_nanos()),
        Utc,
    );
    let mut out: Vec<EntryChunkIndex> = vec![];
    let indexes = TimeIndex::get_indexes_for_time_span(last_seen, now)?;
    for index in indexes {
        let links = get_links(index.hash()?, link_tag.clone())?;
        out.push(EntryChunkIndex {
            time_frame: TimeIndex::try_from(index)?,
            links: links,
        })
    }
    Ok(out)
}

/// Uses sys_time to get links on current time index. Note: this is not guaranteed to return results. It will only look
/// at the current time index which will cover as much time as the current system time - MAX_CHUNK_INTERVAL
pub fn get_current_addresses(
    index: String,
    link_tag: Option<LinkTag>,
    _limit: Option<usize>,
) -> ExternResult<Option<EntryChunkIndex>> {
    match TimeIndex::get_current_index(index)? {
        Some(index) => {
            let links = get_links(index.hash()?, link_tag)?;
            Ok(Some(EntryChunkIndex {
                time_frame: TimeIndex::try_from(index)?,
                links: links,
            }))
        }
        None => Ok(None),
    }
}

/// Searches time index for most recent index and returns links from that index
/// Guaranteed to return results if some index's have been made
pub fn get_most_recent_indexes(
    index: String,
    link_tag: Option<LinkTag>,
    _limit: Option<usize>,
) -> ExternResult<Option<EntryChunkIndex>> {
    let recent_index = TimeIndex::get_latest_index(index)?;
    match recent_index {
        Some(index) => {
            let links = get_links(index.hash()?, link_tag)?;
            Ok(Some(EntryChunkIndex {
                time_frame: TimeIndex::try_from(index)?,
                links: links,
            }))
        },
        None => Ok(None),
    }
}

/// Index a given entry. Uses ['IndexableEntry::entry_time()'] to get time it should be indexed under.
/// Will create link from time path to entry with link_tag passed into fn
pub fn index_entry<T: IndexableEntry, LT: Into<LinkTag>>(
    index: String,
    data: T,
    link_tag: LT,
) -> ExternResult<()> {
    debug!("RECEIVED CALL MAKE CHUNK\n\n\n\n\n\n\n");
    let index = TimeIndex::create_for_timestamp(index, data.entry_time())?;
    create_link(index.hash()?, data.hash()?, link_tag)?;
    Ok(())
}

// /// Set the interval which a time index represents. This should be carefully selected based on link limits set and popularity of DHT.
// pub (crate) fn set_index_interval(interval: Duration) {
//     let mut w = MAX_CHUNK_INTERVAL.write().expect("Could not set MAX_CHUNK_INTERVAL");
//     *w = interval;

//     if interval < Duration::from_secs(1) {
//         let mut tw = TIME_INDEX_DEPTH
//             .write()
//             .expect("Could not set TIME_INDEX_DEPTH");
//         *tw = vec![
//             TimeIndexType::Second,
//             TimeIndexType::Minute,
//             TimeIndexType::Hour,
//             TimeIndexType::Day,
//         ];
//     } else if interval < Duration::from_secs(60) {
//         let mut tw = TIME_INDEX_DEPTH
//             .write()
//             .expect("Could not set TIME_INDEX_DEPTH");
//         *tw = vec![TimeIndexType::Minute, TimeIndexType::Hour, TimeIndexType::Day];
//     } else if interval < Duration::from_secs(3600) {
//         let mut tw = TIME_INDEX_DEPTH
//             .write()
//             .expect("Could not set TIME_INDEX_DEPTH");
//         *tw = vec![TimeIndexType::Hour, TimeIndexType::Day];
//     } else {
//         let mut tw = TIME_INDEX_DEPTH
//             .write()
//             .expect("Could not set TIME_INDEX_DEPTH");
//         *tw = vec![TimeIndexType::Day];
//     };
// }

// /// Set spam protection/DHT hotspot rules. Spam limit determines how many max links a given agent can
// /// create on a time index
// pub (crate) fn set_index_limit(spam_limit: usize) {
//     let mut w = ENFORCE_SPAM_LIMIT
//         .write()
//         .expect("Could not set ENFORCE_SPAM_LIMIT");
//     *w = spam_limit;
// }

// Configuration
// TODO: using rwlock and setter functions does not work in HC since each zome call fn is sandboxed and not a long running bin
// these vars should instead be grabbed from DNA properies. For now these props can just be init with below values.
lazy_static! {
    //Point at which links are considered spam and linked expressions are not allowed
    pub static ref ENFORCE_SPAM_LIMIT: RwLock<usize> = RwLock::new(20);
    //Max duration of given time chunk
    pub static ref MAX_CHUNK_INTERVAL: RwLock<Duration> = RwLock::new(Duration::new(10, 0));
    //Determine what depth of time index chunks should be hung from
    pub static ref TIME_INDEX_DEPTH: RwLock<Vec<entries::TimeIndexType>> = RwLock::new(vec![]);
}
