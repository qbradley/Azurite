# Porting Record — `src/table/generated/artifacts/models.ts`

## File info
- Source path: `src/table/generated/artifacts/models.ts`
- Source lines: `1674`
- Source type: `generated (autorest)`
- Rust target: `azurite-table/src/generated/artifacts/models.rs`
- Crate: `azurite-table`
- Module: `generated::artifacts::models`
- Status: `ported`

## Special handling
Table version of Data models (AccessPolicy, QueueItem, QueueMessage, ListQueuesSegmentResponse, etc.). Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/artifacts/models.md` for detailed fidelity notes.
