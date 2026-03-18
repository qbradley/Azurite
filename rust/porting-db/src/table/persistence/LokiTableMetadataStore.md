# Porting Record — `src/table/persistence/LokiTableMetadataStore.ts`

## File info
- Source path: `src/table/persistence/LokiTableMetadataStore.ts`
- Source lines: `1088`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/loki_table_metadata_store.rs`
- Crate: `azurite-table`
- Module: `persistence::loki_table_metadata_store`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `Loki` | `HashMap-based in-memory store` | D-007: Custom store, not LokiJS port |
| `Collection` | `HashMap<String, Vec<Entity>>` | Per-table entity collections |
| `DynamicView` | `filtered iterator` | Query views |

## Special handling
Largest persistence file. Implements ITableMetadataStore using in-memory storage (Rust uses HashMap+RwLock per D-007, not LokiJS port).

Key fidelity concerns:
1. **ETag generation**: Must match TS format exactly (W/"datetime'...'" format)
2. **Query filtering**: Delegates to LokiTableStoreQueryGenerator for dynamic query construction
3. **Entity property flattening**: Entities stored with flattened properties for query efficiency
4. **Batch atomicity**: Must support rollback of partial batch operations
5. **Timestamp auto-generation**: Server sets Timestamp on every entity write

## Change propagation notes
Changes to entity storage format affect query generation and batch operations. ETag format is part of public API.
