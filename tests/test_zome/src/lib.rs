use chrono::{DateTime, Utc};
use hdk::hash_path::path::Path;
use hdk::prelude::*;

use hc_time_index::*;

#[hdk_entry(id = "test_entry", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Clone)]
pub struct TestEntry {
    pub title: String,
    pub created: DateTime<Utc>,
}

impl IndexableEntry for TestEntry {
    fn entry_time(&self) -> DateTime<Utc> {
        self.created
    }

    fn hash(&self) -> ExternResult<EntryHash> {
        hash_entry(self)
    }
}

entry_defs![Path::entry_def(), TestEntry::entry_def()];

#[hdk_extern]
pub fn init(_: ()) -> ExternResult<InitCallbackResult> {
    Ok(InitCallbackResult::Pass)
}

#[hdk_extern]
pub fn index_entry(entry: TestEntry) -> ExternResult<()> {
    create_entry(&entry)?;
    hc_time_index::index_entry(String::from("test_index"), entry, LinkTag::new("test"))
        .map_err(|err| WasmError::Host(String::from(err)))?;
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
    Ok(hc_time_index::get_indexes_for_time_span(
        input.index,
        input.from,
        input.until,
        input.link_tag,
    )
    .map_err(|err| WasmError::Host(String::from(err)))?)
}

// #[hdk_extern]
// pub fn get_links_for_time_span(input: GetAddressesSinceInput) -> ExternResult<Vec<Link>> {
//     Ok(hc_time_index::get_links_for_time_span(
//         input.index,
//         input.from,
//         input.until,
//         input.link_tag,
//         hc_time_index::SearchStrategy::Dfs,
//         Some(10),
//     )
//     .map_err(|err| WasmError::Host(String::from(err)))?)
// }

#[hdk_extern]
pub fn get_links_and_load_for_time_span(input: GetAddressesSinceInput) -> ExternResult<Vec<TestEntry>> {
    Ok(hc_time_index::get_links_and_load_for_time_span(
        input.index,
        input.from,
        input.until,
        input.link_tag,
        hc_time_index::SearchStrategy::Dfs,
        Some(10),
    )
    .map_err(|err| WasmError::Host(String::from(err)))?)
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
        hc_time_index::get_current_index(input.index, input.link_tag)
            .map_err(|err| WasmError::Host(String::from(err)))?,
    )
}

#[hdk_extern]
pub fn get_most_recent_indexes(
    input: GetCurrentAddressesInput,
) -> ExternResult<Option<EntryChunkIndex>> {
    Ok(
        hc_time_index::get_most_recent_indexes(input.index, input.link_tag)
            .map_err(|err| WasmError::Host(String::from(err)))?,
    )
}

#[hdk_extern]
pub fn remove_index(address: EntryHash) -> ExternResult<()> {
    Ok(hc_time_index::remove_index(address).map_err(|err| WasmError::Host(String::from(err)))?)
}
