# Porting Record — `src/table/batch/TableBatchSubResponse.ts`

## File info
- Source path: `src/table/batch/TableBatchSubResponse.ts`
- Source lines: `~100`
- Source type: `handwritten`
- Rust target: `azurite-table/src/handlers/table_batch_sub_response.rs`
- Crate: `azurite-table`
- Module: `handlers::table_batch_sub_response`
- Status: `ported`

## Special handling
Represents the response for a single batch sub-operation. Serialized into multipart MIME response body.
