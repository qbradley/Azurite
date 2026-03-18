# Porting Record — `src/table/middleware/tableStorageContext.middleware.ts`

## File info
- Source path: `src/table/middleware/tableStorageContext.middleware.ts`
- Source lines: `120`
- Source type: `handwritten`
- Rust target: `azurite-table/src/middlewares/table_storage_context_middleware.rs`
- Crate: `azurite-table`
- Module: `middlewares::table_storage_context_middleware`
- Status: `ported`

## Special handling
Extracts table name, partition key, row key from URL path. Table URLs can include entity addressing: /accountname/tablename(PartitionKey='pk',RowKey='rk'). OData entity key parsing is table-specific.
