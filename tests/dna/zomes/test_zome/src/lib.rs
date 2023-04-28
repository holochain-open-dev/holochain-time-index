use chrono::{DateTime, Utc};
use hdk::prelude::*;
use test_zome_integrity::{TestEntry, EntryTypes, LinkTypes};

use hc_time_index::*;

mod utils;

#[hdk_extern]
pub fn init(_: ()) -> ExternResult<InitCallbackResult> {
    Ok(InitCallbackResult::Pass)
}

#[hdk_extern]
pub fn index_entry(entry: TestEntry) -> ExternResult<()> {
    create_entry(&EntryTypes::TestEntry(entry.clone()))?;
    hc_time_index::index_entry(String::from("test_index"), entry, LinkTag::new("test"), LinkTypes::Index, LinkTypes::Path)
        .map_err(|error| utils::err(&format!("{}", error)))?;
    Ok(())
}

#[derive(Serialize, Deserialize, SerializedBytes, Debug)]
pub struct GetAddressesSinceInput {
    pub index: String,
    pub from: DateTime<Utc>,
    pub until: DateTime<Utc>,
    pub limit: Option<usize>,
    pub link_tag: Option<LinkTag>,
}

#[hdk_extern]
pub fn get_indexes_for_time_span(
    input: GetAddressesSinceInput,
) -> ExternResult<Vec<hc_time_index::EntryChunkIndex>> {
    hc_time_index::get_indexes_for_time_span(
        input.index,
        input.from,
        input.until,
        input.link_tag,
        LinkTypes::Index,
        LinkTypes::Path
    )
    .map_err(|error| utils::err(&format!("{}", error)))
}

#[hdk_extern]
pub fn get_links_for_time_span(input: GetAddressesSinceInput) -> ExternResult<Vec<Link>> {
    Ok(hc_time_index::get_links_for_time_span(
        input.index,
        input.from,
        input.until,
        input.link_tag,
        Some(10),
        LinkTypes::Index,
        LinkTypes::Path
    )
    .map_err(|error| utils::err(&format!("{}", error)))?)
}

#[hdk_extern]
pub fn get_links_and_load_for_time_span(input: GetAddressesSinceInput) -> ExternResult<Vec<TestEntry>> {
    Ok(hc_time_index::get_links_and_load_for_time_span(
        input.index,
        input.from,
        input.until,
        input.link_tag,
        hc_time_index::SearchStrategy::Dfs,
        Some(10),
        LinkTypes::Index,
        LinkTypes::Path
    )
    .map_err(|error| utils::err(&format!("{}", error)))?)
}

#[derive(Serialize, Deserialize, SerializedBytes, Debug)]
pub struct GetCurrentAddressesInput {
    pub index: String,
    pub limit: Option<usize>,
    pub link_tag: Option<LinkTag>,
}

#[hdk_extern]
pub fn get_current_addresses(
    input: GetCurrentAddressesInput,
) -> ExternResult<Option<EntryChunkIndex>> {
    Ok(
        hc_time_index::get_current_index(input.index, input.link_tag, LinkTypes::Path)
            .map_err(|error| utils::err(&format!("{}", error)))?,
    )
}

#[hdk_extern]
pub fn remove_index(address: EntryHash) -> ExternResult<()> {
    Ok(hc_time_index::remove_index(address, LinkTypes::Index).map_err(|error| utils::err(&format!("{}", error)))?)
}
