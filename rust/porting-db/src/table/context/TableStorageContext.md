# Porting Record — `src/table/context/TableStorageContext.ts`

## File info
- Source path: `src/table/context/TableStorageContext.ts`
- Source lines: `~80`
- Source type: `handwritten`
- Rust target: `azurite-table/src/context/table_storage_context.rs`
- Crate: `azurite-table`
- Module: `context::table_storage_context`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `TableStorageContext` | `TableStorageContext struct` | Request context |
| `string` | `String` | Table/account/entity names |

## Special handling
Request context carrying table name, partition key, row key, account info. Parallel to queue/blob context pattern but includes entity-level addressing (partitionKey + rowKey in URL path).
