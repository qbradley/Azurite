# Porting Record — `src/table/persistence/LokiTableStoreQueryGenerator.ts`

## File info
- Source path: `src/table/persistence/LokiTableStoreQueryGenerator.ts`
- Source lines: `99`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/loki_table_store_query_generator.rs`
- Crate: `azurite-table`
- Module: `persistence::loki_table_store_query_generator`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `LokiJS query object` | `closure-based filter` | Query generation |

## Special handling
Translates OData $filter expressions into store query predicates. Works with QueryInterpreter to evaluate filters against in-memory entities. Table-specific — no blob equivalent.
