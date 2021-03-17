//! # Holochain-Time-Index
//!
//! ## Purpose
//!
//! This DHT aims to be one solution (of many) to the DHT hostpotting problem that can occur in holochain DHT's when many links are made from one entry.
//! This hotspotting occurs as the original author (and their surrounding hash neighbourhood?) of an entry is responsible for storing and resolving all links from the given authored entry. As a result if a given entry becomes very popular then it can be left up to one or a few nodes to handle all traffic flowing through this part of the DHT.
//!
//! ## Function
//!
//! The main component that allows the mitigation of DHT hotspots are:
//! 1) time delimited indexing.
//! 2) agent focused validation that occurs on each index.
//!
//! ### Time Delimited Indexing
//!
//! This crate exposes an `index_entry(index: String, entry: T, link_tag: Into<LinkTag>)` function. This function indexes the submitted entry into a time b-tree. The b-tree looks something like the following:
//!
//! ![B-tree](https://github.com/holochain-open-dev/Holochain-Time-Index)
//!
//! In the above example we are indexing 3 entries. It should be simple to follow the time tree and see how this tree can be used to locate an entry in time; but we have also introduced a new concept: TimeFrame.
//! TimeFrame is the last piece of the path where entries get linked. This allows for the specification of a time frame that is greater than one unit of the "parent" time. This is useful when you want to link at a fidelity that is not offered by the ordinary time data; i.e index links at every 30 second chunk vs every minute or link to every 10 minute chunk vs every hour.
//! This time frame can be set by adding the `ENFORCE_SPAM_LIMIT` to host DNA's properties.
//!
//! Indexes into time tree occur based on the value received from `IndexableEntry::entry_time(&self)` trait function that should be derive on the entry type you wish to index.
//!
//! ### Agent Link Validation
//!
//! For any given index an **agent** cannot make more than `ENFORCE_SPAM_LIMIT` links. This value is set by the properties of the host DNA which is using this library; this library will just read host DNA's properties and set its internal variables based on what it finds.
//!
//! ### DNA Lifecycle
//!
//! This DNA's variables mentioned above are expected to be static. That means its expected that the: `ENFORCE_SPAM_LIMIT` & `MAX_CHUNK_INTERVAL` should stay the same throughout the lifetime of the DHT. This is done to make validation possible in situations where DHT sharding could occur.
//! If limits are able to change; we have no way to reliably know if an agent is operating on old limits by consequence of being out of touch with latest DHT state or if the agent is malicious and pretending they do not see the new limits. You can see this being an especially big problem when you have two areas of the DHT "merging" and the "outdated" area of the DHT having all of its links in-validated by the agents in the more current of the DHT space.
//!
//! Currently if we wish to update limits we will create a new DNA/DHT and link to the new one from the current.
//!
//! If you can guarantee that fragmentation of the DHT will not happen then its possible to implement limit updates. If this is something you wish to do its recommended that you enforce new limits at some given chunk in the future rather than instantly. This allows you to (hopefully) give enough time for other DHT agents to receive new limit information before its enforced.   
//!
//! ### Exposed Functions
//!
//! This DNA exposes a few helper functions to make integrating with this time series data easy. Functions are:
//!
//! `get_indexes_between()`: Gets links between two time periods
//! `get_current_index()`: Gets links on current index period
//! `get_most_recent_indexes()`: Gets the most recent links
//! `index_entry()`: Indexes an entry into time tree
//!
//! ### hApp Usage
//!
//! Using the above methods its possible to build an application which places an emphasis on time ordered data (such as a group DM or news feed). Or you can use the time ordered nature of the data as a natural pagination for larger queries where you may wish to aggregate data over a given time period and then perform some further computations over it.
//!
//!
//! ## Status
//!
//! - [x] Basic public lib functions implemented & tested
//! - [x] Basic performance optimizations for search functions
//! - [ ] Advanced Performance optimizations for search functions
//! - [ ] Advanced testing of DNA functioning
//! - [ ] Lib's variables derived from host DNA properties (blocked until HDK support)
//! - [ ] Validation functions for links made at indexes
//! - [ ] Validation functions for time b-tree shape & structure
//! - [ ] Limit of returned links in public functions
//!
//! ### Limitations
//!
//! - You cannot index at time before UNIX epoch (00:00:00 UTC on 1 January 1970)
//! - Limit & interval variables must be static throughout lifetime of DHT
//! - Calling `get_indexes_between()` with a large from & until value will take a long time to return
//! - It is currently not possible to set library variables by adding appropriate variables to DHT properties. This crate must instead be forked, altered and then used inside your DNA.

#[macro_use]
extern crate lazy_static;

use chrono::{DateTime, Utc};
use std::sync::RwLock;
use std::time::Duration;

use hdk3::prelude::*;

pub mod errors;
mod impls;
mod search;
mod utils;
mod validation;

/// Public methods exposed by lib
pub mod methods;

/// All holochain entries used by this crate
pub mod entries;

mod traits;
/// Trait to impl on entries that you want to add to time index
pub use traits::IndexableEntry;

use entries::{Index, IndexType};
use errors::{IndexError, IndexResult};
use utils::unwrap_chunk_interval_lock;

#[derive(Serialize, Deserialize, Debug)]
pub struct EntryChunkIndex {
    pub index: Index,
    pub links: Links,
}

/// Gets all links with optional tag link_tag since last_seen time with option to limit number of results by limit
/// Note: if last_seen is a long time ago in a popular DHT then its likely this function will take a very long time to run
/// TODO: would be cool to support DFS and BFS here
pub fn get_indexes_for_time_span(
    index: String,
    from: DateTime<Utc>,
    until: DateTime<Utc>,
    _limit: Option<usize>,
    link_tag: Option<LinkTag>,
) -> IndexResult<Vec<EntryChunkIndex>> {
    let max_chunk_interval = unwrap_chunk_interval_lock();
    //Check that timeframe specified is greater than the TIME_INDEX_DEPTH.
    if until.timestamp_millis() - from.timestamp_millis() < max_chunk_interval.as_millis() as i64 {
        return Err(IndexError::RequestError(
            "Time frame is smaller than index interval",
        ));
    };

    Ok(methods::get_indexes_for_time_span(
        from, until, index, link_tag,
    )?)
}

/// Get links for index that exist between two timestamps
pub fn get_links_for_time_span(
    index: String,
    from: DateTime<Utc>,
    until: DateTime<Utc>,
    _limit: Option<usize>,
    link_tag: Option<LinkTag>,
) -> IndexResult<Vec<Link>> {
    let max_chunk_interval = unwrap_chunk_interval_lock();
    //Check that timeframe specified is greater than the TIME_INDEX_DEPTH.
    if until.timestamp_millis() - from.timestamp_millis() < max_chunk_interval.as_millis() as i64 {
        return Err(IndexError::RequestError(
            "Time frame is smaller than index interval",
        ));
    };

    Ok(methods::get_links_for_time_span(
        from, until, index, link_tag,
    )?)
}

/// Get links for index that exist between two timestamps and attempt to serialize link targets to T
pub fn get_links_and_load_for_time_span<
    T: TryFrom<SerializedBytes, Error = SerializedBytesError> + IndexableEntry,
>(
    index: String,
    from: DateTime<Utc>,
    until: DateTime<Utc>,
    _limit: Option<usize>,
    link_tag: Option<LinkTag>,
) -> IndexResult<Vec<T>> {
    let max_chunk_interval = unwrap_chunk_interval_lock();
    //Check that timeframe specified is greater than the TIME_INDEX_DEPTH.
    if until.timestamp_millis() - from.timestamp_millis() < max_chunk_interval.as_millis() as i64 {
        return Err(IndexError::RequestError(
            "Time frame is smaller than index interval",
        ));
    };

    Ok(methods::get_links_and_load_for_time_span::<T>(
        from, until, index, link_tag,
    )?)
}

/// Uses sys_time to get links on current time index. Note: this is not guaranteed to return results. It will only look
/// at the current time index which will cover as much time as the current system time - MAX_CHUNK_INTERVAL
pub fn get_current_index(
    index: String,
    link_tag: Option<LinkTag>,
    _limit: Option<usize>,
) -> IndexResult<Option<EntryChunkIndex>> {
    match methods::get_current_index(index)? {
        Some(index) => {
            let links = get_links(index.hash()?, link_tag)?;
            Ok(Some(EntryChunkIndex {
                index: Index::try_from(index)?,
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
) -> IndexResult<Option<EntryChunkIndex>> {
    let recent_index = methods::get_latest_index(index)?;
    match recent_index {
        Some(index) => {
            let links = get_links(index.hash()?, link_tag)?;
            Ok(Some(EntryChunkIndex {
                index: Index::try_from(index)?,
                links: links,
            }))
        }
        None => Ok(None),
    }
}

/// Index a given entry. Uses ['IndexableEntry::entry_time()'] to get time it should be indexed under.
/// Will create link from time path to entry with link_tag passed into fn
pub fn index_entry<T: IndexableEntry, LT: Into<LinkTag>>(
    index: String,
    data: T,
    link_tag: LT,
) -> IndexResult<()> {
    let index = methods::create_for_timestamp(index, data.entry_time())?;
    create_link(index.hash()?, data.hash()?, link_tag)?;
    Ok(())
}

/// Returns the child paths on submitted paths. This allows the manual traversal down a time tree to get results as desired by callee
pub fn get_paths_for_path(path: Path, link_tag: Option<LinkTag>) -> IndexResult<Vec<Path>> {
    match link_tag {
        Some(link_tag) => Ok(get_links(path.hash()?, Some(link_tag))?
            .into_inner()
            .into_iter()
            .map(|link| Ok(Path::try_from(&link.tag)?))
            .collect::<IndexResult<Vec<Path>>>()?),
        None => Ok(path
            .children()?
            .into_inner()
            .into_iter()
            .map(|link| Ok(Path::try_from(&link.tag)?))
            .collect::<IndexResult<Vec<Path>>>()?),
    }
}

// Configuration
// TODO: using rwlock and setter functions does not work in HC since each zome call fn is sandboxed and not a long running bin
// these vars should instead be grabbed from DNA properies. For now these props can just be init with below values.
lazy_static! {
    //Point at which links are considered spam and linked expressions are not allowed
    pub static ref ENFORCE_SPAM_LIMIT: RwLock<usize> = RwLock::new(20);
    //Max duration of given time chunk
    pub static ref MAX_CHUNK_INTERVAL: RwLock<Duration> = RwLock::new(Duration::new(100, 0));
    //Determine what depth of time index should be hung from
    pub static ref TIME_INDEX_DEPTH: RwLock<Vec<entries::IndexType>> = RwLock::new(
        if *MAX_CHUNK_INTERVAL.read().expect("Could not get read for MAX_CHUNK_INTERVAL") < Duration::from_secs(1) {
            vec![
                IndexType::Second,
                IndexType::Minute,
                IndexType::Hour,
                IndexType::Day,
            ]
        } else if *MAX_CHUNK_INTERVAL.read().expect("Could not get read for MAX_CHUNK_INTERVAL") < Duration::from_secs(60) {
            vec![IndexType::Minute, IndexType::Hour, IndexType::Day]
        } else if *MAX_CHUNK_INTERVAL.read().expect("Could not get read for MAX_CHUNK_INTERVAL") < Duration::from_secs(3600) {
            vec![IndexType::Hour, IndexType::Day]
        } else {
            vec![IndexType::Day]
        }
    );
}
