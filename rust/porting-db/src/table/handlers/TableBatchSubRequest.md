# Porting Record — `src/table/batch/TableBatchSubRequest.ts`

## File info
- Source path: `src/table/batch/TableBatchSubRequest.ts`
- Source lines: `~150`
- Source type: `handwritten`
- Rust target: `azurite-table/src/handlers/table_batch_sub_request.rs`
- Crate: `azurite-table`
- Module: `handlers::table_batch_sub_request`
- Status: `ported`

## Special handling
Represents a single operation within a batch changeset. Parsed from multipart MIME body.
