//! # Holochain-Time-Index
//!
//! ## Purpose
//!
//! This DHT aims to be one solution (of many) to the DHT hotspotting problem that can occur in holochain DHT's when many links are made from one entry.
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
//! This crate exposes an `index_entry(index: String, entry: T, link_tag: Into<LinkTag>, index_link_type: ILT, path_link_type: PLT)` function. This function indexes the submitted entry into a time b-tree. The b-tree looks something like the following:
//!
//! ![B-tree](https://github.com/holochain-open-dev/holochain-time-index/tree/main/media/b-tree-time-path.png)
//!
//! In the above example we are indexing 3 entries. It should be simple to follow the time tree and see how this tree can be used to locate an entry in time; but we have also introduced a new concept: TimeFrame.
//! TimeFrame is the last piece of the path where entries get linked. This allows for the specification of a time frame that is greater than one unit of the "parent" time. This is useful when you want to link at a fidelity that is not offered by the ordinary time data; i.e index links at every 30 second chunk vs every minute or link to every 10 minute chunk vs every hour.
//! This time frame can be set by adding the `MAX_CHUNK_INTERVAL` to host DNA's properties.
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
//! - `get_indexes_between()`: Gets links between two time periods
//! - `get_current_index()`: Gets links on current index period
//! - `get_most_recent_indexes()`: Gets the most recent links
//! - `index_entry()`: Indexes an entry into time tree
//! 
//! Many of the above functions require `index_link_type` & `path_link_type` values to be provided. These should be defined `LinkTypes` in your happs integrity zome. The `index_link_type` is the link type that gets used when creating links between the time tree and the entry you wish to index. 
//! The `path_link_type` is the link type which is used when creating links between Path entries (time tree entries). By leveraging different LinkTypes for different indexes it would be possible to create multiple index trees. 
//!
//! ### hApp Usage
//!
//! Using the above methods, it's possible to build an application which places an emphasis on time ordered data (such as a group DM or news feed). Or you can use the time ordered nature of the data as a natural pagination for larger queries where you may wish to aggregate data over a given time period and then perform some further computations over it.
//!
//! ### Compatibility
//!
//! This crate has been built to work with HDK version 0.0.139 & holochain_deterministic_integrity 0.0.11
//!
//! ## Status
//!
//! - [x] Basic public lib functions implemented & tested
//! - [x] Basic performance optimizations for search functions
//! - [ ] Advanced Performance optimizations for search functions
//! - [ ] Advanced testing of DNA functioning
//! - [x] Lib's variables derived from host DNA properties (blocked until HDK support)
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
use std::time::Duration;

use hdi::prelude::*;
use hdk::prelude::*;

mod bfs;
mod convertions;
mod dfs;
pub mod errors;
mod impl_utils;

/// Public methods exposed by lib
pub mod methods;
mod search;
mod traits;
mod utils;
mod validation;

/// All holochain entries used by this crate
pub mod entries;

/// Trait to impl on entries that you want to add to time index
pub use traits::IndexableEntry;

use entries::{Index, IndexType};
use errors::{IndexError, IndexResult};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EntryChunkIndex {
    pub index: Index,
    pub links: Vec<Link>,
}

/// Configuration object that should be set in your host DNA's properties
#[derive(Serialize, Deserialize, Debug, SerializedBytes)]
pub struct IndexConfiguration {
    pub enforce_spam_limit: usize,
    pub max_chunk_interval: usize,
}

pub enum SearchStrategy {
    Dfs,
    Bfs,
}

#[derive(Debug)]
pub(crate) enum Order {
    Desc,
    Asc,
}

/// Gets all links with optional tag link_tag since last_seen time with option to limit number of results by limit
/// Note: if last_seen is a long time ago in a popular DHT then its likely this function will take a very long time to run
/// TODO: would be cool to support DFS and BFS here
pub fn get_indexes_for_time_span<PLT: Clone>(
    index: String,
    from: DateTime<Utc>,
    until: DateTime<Utc>,
    link_tag: Option<LinkTag>,
    index_link_type: impl LinkTypeFilterExt + Clone,
    path_link_type: PLT
) -> IndexResult<Vec<EntryChunkIndex>> 
    where ScopedLinkType: TryFrom<PLT, Error = WasmError> {
    //Check that timeframe specified is greater than the INDEX_DEPTH.
    if until.timestamp_millis() - from.timestamp_millis() < MAX_CHUNK_INTERVAL.as_millis() as i64 {
        return Err(IndexError::RequestError(
            "Time frame is smaller than index interval",
        ));
    };

    Ok(methods::get_indexes_for_time_span(
        from, until, index, link_tag, index_link_type, path_link_type
    )?)
}

/// Get links for index that exist between two timestamps
pub fn get_links_for_time_span<PLT: Clone>(
    index: String,
    from: DateTime<Utc>,
    until: DateTime<Utc>,
    link_tag: Option<LinkTag>,
    limit: Option<usize>,
    index_link_type: impl LinkTypeFilterExt + Clone,
    path_link_type: PLT
) -> IndexResult<Vec<Link>> 
    where ScopedLinkType: TryFrom<PLT, Error = WasmError> {
    // //Check that timeframe specified is greater than the INDEX_DEPTH.
    // if until.timestamp_millis() - from.timestamp_millis() < MAX_CHUNK_INTERVAL.as_millis() as i64 {
    //     return Err(IndexError::RequestError(
    //         "Time frame is smaller than index interval",
    //     ));
    // };

    Ok(methods::get_links_for_time_span(
        index, from, until, link_tag, limit, index_link_type, path_link_type
    )?)
}

/// Get links for index that exist between two timestamps and attempt to serialize link targets to T
pub fn get_links_and_load_for_time_span<
    T: TryFrom<SerializedBytes, Error = SerializedBytesError> + IndexableEntry + std::fmt::Debug,
    ILT: LinkTypeFilterExt + Clone,
    PLT: Clone
>(
    index: String,
    from: DateTime<Utc>,
    until: DateTime<Utc>,
    link_tag: Option<LinkTag>,
    strategy: SearchStrategy,
    limit: Option<usize>,
    index_link_type: ILT,
    path_link_type: PLT
) -> IndexResult<Vec<T>> 
    where ScopedLinkType: TryFrom<PLT, Error = WasmError> {
    // //Check that timeframe specified is greater than the INDEX_DEPTH.
    // if until.timestamp_millis() - from.timestamp_millis() < MAX_CHUNK_INTERVAL.as_millis() as i64 {
    //     return Err(IndexError::RequestError(
    //         "Time frame is smaller than index interval",
    //     ));
    // };

    Ok(methods::get_links_and_load_for_time_span::<T, ILT, PLT>(
        from, until, index, link_tag, strategy, limit, index_link_type, path_link_type
    )?)
}

/// Uses sys_time to get links on current time index. Note: this is not guaranteed to return results. It will only look
/// at the current time index which will cover as much time as the current system time - MAX_CHUNK_INTERVAL
pub fn get_current_index<PLT: Clone + LinkTypeFilterExt>(
    index: String,
    link_tag: Option<LinkTag>,
    path_link_type: PLT
) -> IndexResult<Option<EntryChunkIndex>> 
    where ScopedLinkType: TryFrom<PLT, Error = WasmError> {
    match methods::get_current_index(index, path_link_type.clone())? {
        Some(index) => {
            let links = get_links(index.path_entry_hash()?, path_link_type, link_tag)?;
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
pub fn index_entry<T: IndexableEntry, LT: Into<LinkTag>, ILT: Clone, PLT>(
    index: String,
    data: T,
    link_tag: LT,
    index_link_type: ILT,
    path_link_type: PLT
) -> IndexResult<()> 
    where ScopedLinkType: TryFrom<ILT, Error = WasmError> + TryFrom<PLT, Error = WasmError> {
    let index = methods::create_for_timestamp(index, data.entry_time(), path_link_type)?;
    //Create link from end of time path to entry that should be indexed
    create_link(index.path_entry_hash()?, data.hash()?, index_link_type.clone(), link_tag)?;
    //Create link from entry that should be indexed back to time tree so tree links can be found when starting from entry
    create_link(data.hash()?, index.path_entry_hash()?, index_link_type, LinkTag::new("time_path"))?;
    Ok(())
}

/// Removes a given indexed entry from the time tree
pub fn remove_index(indexed_entry: EntryHash, index_link_type: impl LinkTypeFilterExt + Clone) -> IndexResult<()> {
    let time_paths =
        get_links(indexed_entry.clone(), index_link_type.clone(), Some(LinkTag::new("time_path")))?;
    for time_path in time_paths {
        let path_links = get_links(time_path.target.clone(), index_link_type.clone(), None)?;
        let path_links: Vec<Link> = path_links
            .into_iter()
            .filter(|link| EntryHash::from(link.target.to_owned()) == indexed_entry)
            .collect();
        for path_link in path_links {
            // debug!(
            //     "Deleting link: {:#?}",
            //     path_link.create_link_hash.to_owned()
            // );
            delete_link(path_link.create_link_hash.to_owned())?;
        }
    }

    Ok(())
}

// Library configuration setup
lazy_static! {
    //Point at which links are considered spam and linked expressions are not allowed
    pub static ref ENFORCE_SPAM_LIMIT: usize = {
        // debug!("Attempting to set spam limit from: {:#?}", zome_info());
        let host_dna_config = dna_info().expect("Could not get zome configuration").properties;
        let properties = IndexConfiguration::try_from(host_dna_config)
            .expect("Could not convert zome dna properties to IndexConfiguration. Please ensure that your dna properties contains a IndexConfiguration field.");
        properties.enforce_spam_limit
    };
    pub static ref MAX_CHUNK_INTERVAL: Duration = {
        let host_dna_config = dna_info().expect("Could not get zome configuration").properties;
        let properties = IndexConfiguration::try_from(host_dna_config)
            .expect("Could not convert zome dna properties to IndexConfiguration. Please ensure that your dna properties contains a IndexConfiguration field.");
        Duration::from_millis(properties.max_chunk_interval as u64)
    };
    //Determine what depth of time index should be hung from
    pub static ref INDEX_DEPTH: Vec<entries::IndexType> =
        if *MAX_CHUNK_INTERVAL < Duration::from_secs(1) {
            vec![
                IndexType::Second,
                IndexType::Minute,
                IndexType::Hour,
                IndexType::Day,
            ]
        } else if *MAX_CHUNK_INTERVAL < Duration::from_secs(60) {
            vec![IndexType::Minute, IndexType::Hour, IndexType::Day]
        } else if *MAX_CHUNK_INTERVAL < Duration::from_secs(3600) {
            vec![IndexType::Hour, IndexType::Day]
        } else {
            vec![IndexType::Day]
        };

    pub static ref DEFAULT_INDEX_DEPTH: Vec<IndexType> = vec![IndexType::Second,
        IndexType::Month,
        IndexType::Year
    ];
}
