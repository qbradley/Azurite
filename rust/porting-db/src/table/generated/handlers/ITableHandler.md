# Porting Record — `src/table/generated/handlers/ITableHandler.ts`

## File info
- Source path: `src/table/generated/handlers/ITableHandler.ts`
- Source lines: `~50`
- Source type: `generated framework`
- Rust target: `azurite-table/src/generated/handlers/i_table_handler.rs`
- Crate: `azurite-table`
- Module: `generated::handlers::i_table_handler`
- Status: `ported`

## Special handling
Table-level handler trait covering all entity/table operations. Table-specific — no direct blob equivalent. Includes batch, query, insert, update, merge, delete operations.
