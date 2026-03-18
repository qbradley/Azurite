# Porting Record — `src/table/handlers/TableHandler.ts`

## File info
- Source path: `src/table/handlers/TableHandler.ts`
- Source lines: `1188`
- Source type: `handwritten`
- Rust target: `azurite-table/src/handlers/table_handler.rs`
- Crate: `azurite-table`
- Module: `handlers::table_handler`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `ITableMetadataStore` | `Arc<dyn ITableMetadataStore>` | Metadata store |
| `Context` | `&Context` | Request context |
| `Models.*` | `generated models` | Request/response types |

## Special handling
Largest handler — covers all table and entity operations:
- Table CRUD: createTable, deleteTable, queryTables
- Entity CRUD: insertEntity, deleteEntity, updateEntity, mergeEntity, queryEntities, queryEntitiesWithPartitionAndRowKey
- Response serialization with OData annotation levels (Full/Minimal/No)
- ETag generation and conditional request handling (If-Match)

Table-specific — no direct blob equivalent. This is the primary handler for the Table service.
