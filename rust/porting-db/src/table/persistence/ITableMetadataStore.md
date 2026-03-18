# Porting Record — `src/table/persistence/ITableMetadataStore.ts`

## File info
- Source path: `src/table/persistence/ITableMetadataStore.ts`
- Source lines: `141`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/i_table_metadata_store.rs`
- Crate: `azurite-table`
- Module: `persistence::i_table_metadata_store`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `interface ITableMetadataStore` | `trait ITableMetadataStore` | Async trait |
| `Promise<T>` | `async fn -> Result<T>` | Async operations |
| `Entity` | `Entity type` | Table entity model |

## Special handling
Core persistence trait for table metadata store. Methods include:
- Table CRUD: createTable, deleteTable, queryTable, setTableACL, getTableAccessPolicy
- Entity CRUD: insertTableEntity, deleteTableEntity, updateTableEntity, mergeTableEntity
- Query: queryTableEntities, queryTableEntitiesWithPartitionAndRowKey
- Batch: insertOrUpdateTableEntity, insertOrMergeTableEntity
- Lifecycle: init(), close()

All methods are async. Entity operations must handle ETag-based optimistic concurrency.
