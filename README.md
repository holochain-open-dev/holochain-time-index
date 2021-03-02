# HC-Time-Chunking

## Purpose

This DHT aims to be one solution (of many) to the DHT hostpotting problem that can occur in holochain DHT's when many links are made from one entry.
This hotspotting occurs as the original author (and their surrounding hash neighbourhood?) of an entry is responsible for storing and resolving
all links from the given authored entry. As a result if a given entry becomes very popular then it can be left up to one or a few nodes
to handle all traffic flowing through this part of the DHT.

## Function

The main component that allows the mitigation of DHT hotspots is 1) the time delimited chunks and 2) the agent focused validation that occurs on each chunk. 
For any given chunk an agent cannot make more than `DIRECT_CHUNK_LINK_LIMIT` direct links on a given chunk. Once this limit has been met, subsequent 
links must be linked together in a linked list shape. Here the target entry of the last direct link they created is the source entry of the linked list. 
An agent can make links like this until their total links reaches the `ENFORCE_SPAM_LIMIT` limit at which point no further links are allowed. 
The first limit is a measure to protect DHT hotspots in a busy DHT with a high `MAX_CHUNK_INTERVAL` & the second limit is supposed to block clear/obvious spam.

This DNA's variables is expected to be static. That means its expected that the: `DIRECT_CHUNK_LINK_LIMIT`, `ENFORCE_SPAM_LIMIT` & `MAX_CHUNK_INTERVAL` will 
stay the same throughout the lifetime of the DHT. This is done to make validation possible in situations where DHT could occur. 
If limits are able to change; we have no way to reliably know if an agent is operating on old limits by consequence of being out of touch 
with latest DHT state or if the agent is malicious and pretending they do not see the new limits.
If you can guarantee that fragmentation of the DHT will not happen then its possible that limit updates could work. 
We may also add support for the reduction of limits in the future; but increasing limits will not land.

For now if one wishes to increase these values its recommended to create a new DNA/DHT and link to it from the current. 

This DNA exposes a few helper functions to make operating with chunked data easy. Ones of note are: 
`get_current_chunk()`, `get_latest_chunk()`, `get_chunks_for_time_span()`, `add_link()` & `get_links()`

`get_current_chunk()` will take the current time as denoted by sys_time() and return null or a chunk that can be used to served entries for the current time.<br>
`get_latest_chunk()` will search though the DNA's time "index" and find the last commited chunk and return it.<br>
`get_chunks_for_time_span()` will return all chunks that served in a given time span.<br>
`add_link()` will create a link on a chunk. This will happen either directly or by the linked list fashion as explained above.<br>
`get_links()` will get links from the chunk, recursing down any linked lists to ensure that all links are returned for a given chunk.<br>