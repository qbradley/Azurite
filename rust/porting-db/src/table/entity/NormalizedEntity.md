# Porting Record — `src/table/entity/NormalizedEntity.ts`

## File info
- Source path: `src/table/entity/NormalizedEntity.ts`
- Source lines: `~100`
- Source type: `handwritten`
- Rust target: `azurite-table/src/entity/normalized_entity.rs`
- Crate: `azurite-table`
- Module: `entity::normalized_entity`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `NormalizedEntity` | `NormalizedEntity struct` | Entity container |
| `Map<string, EntityProperty>` | `HashMap<String, EntityProperty>` | Properties map |

## Special handling
Row-level entity container with PartitionKey, RowKey, eTag, Timestamp, and property map. `to_response_string()` serializes to OData JSON with correct annotation level. `from_entity()` converts from persistence model.
