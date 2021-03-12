use chrono::{DateTime, Utc};
use hdk3::prelude::*;
use hdk3::hash_path::path::Path;

use hc_time_index::*;

#[hdk_entry(id = "test_entry", visibility = "public")]
#[serde(rename_all = "camelCase")]
#[derive(Clone)]
pub struct TestEntry {
    pub title: String,
    pub created: DateTime<Utc>
}

impl IndexableEntry for TestEntry {
    fn entry_time(&self) -> DateTime<Utc> {
        self.created
    }
    
    fn hash(&self) -> ExternResult<EntryHash> {
        hash_entry(self)
    }
}

entry_defs![
    Path::entry_def(),
    TestEntry::entry_def()
];

#[hdk_extern]
pub fn init(_: ()) -> ExternResult<InitCallbackResult> {
    Ok(InitCallbackResult::Pass)
}

#[hdk_extern]
pub fn index_entry(entry: TestEntry) -> ExternResult<()> {
    create_entry(&entry)?;
    hc_time_index::index_entry(String::from("test_index"), entry, LinkTag::new("test"))?;
    Ok(())
}

#[derive(Serialize, Deserialize, SerializedBytes, Debug)]
pub struct GetAddressesSinceInput {
    pub index: String,
    pub from: DateTime<Utc>,
    pub until: DateTime<Utc>,
    pub limit: Option<usize>,
    pub link_tag: Option<LinkTag>
}

#[hdk_extern]
pub fn get_addresses_between(
    input: GetAddressesSinceInput
) -> ExternResult<Vec<hc_time_index::EntryChunkIndex>> {
    Ok(hc_time_index::get_indexes_for_time_span(input.index, input.from, input.until, input.limit, input.link_tag)?)
}

#[derive(Serialize, Deserialize, SerializedBytes, Debug)]
pub struct GetCurrentAddressesInput {
    pub index: String,
    pub limit: Option<usize>,
    pub link_tag: Option<LinkTag>
}

#[hdk_extern]
pub fn get_current_addresses(input: GetCurrentAddressesInput) -> ExternResult<Option<EntryChunkIndex>> {
    Ok(hc_time_index::get_current_index(input.index, input.link_tag, input.limit)?)
}

#[hdk_extern]
pub fn get_most_recent_indexes(input: GetCurrentAddressesInput) -> ExternResult<Option<EntryChunkIndex>> {
    Ok(hc_time_index::get_most_recent_indexes(input.index, input.link_tag, input.limit)?)
}
