# Porting Record — `src/table/utils/utils.ts`

## File info
- Source path: `src/table/utils/utils.ts`
- Source lines: `308`
- Source type: `handwritten`
- Rust target: `azurite-table/src/utils/utils.rs`
- Crate: `azurite-table`
- Module: `utils::utils`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `Date` | `DateTime<Utc>` | Timestamp handling |
| `any` | `serde_json::Value` | Dynamic JSON values |

## Special handling
Table utility functions including:
- Entity property serialization helpers
- OData URL path parsing (entity key extraction from parenthesized notation)
- ETag generation and validation
- Table name validation
- Timestamp formatting (ISO 8601 with specific precision)

Table-specific URL parsing is unique: `tablename(PartitionKey='pk',RowKey='rk')` format.
