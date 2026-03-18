# Porting Record — `src/table/handlers/TableBatchHandler.ts`

## File info
- Source path: `src/table/handlers/TableBatchHandler.ts`
- Source lines: `~300`
- Source type: `handwritten`
- Rust target: `azurite-table/src/handlers/table_batch_handler.rs`
- Crate: `azurite-table`
- Module: `handlers::table_batch_handler`
- Status: `ported`

## Special handling
Handles $batch endpoint. Parses multipart request, delegates to TableBatchOrchestrator, serializes multipart response. Table-specific — no direct blob equivalent (blob batch uses JSON).
