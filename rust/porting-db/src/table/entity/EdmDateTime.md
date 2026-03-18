# Porting Record — `src/table/entity/EdmDateTime.ts`

## File info
- Source path: `src/table/entity/EdmDateTime.ts`
- Source lines: `71`
- Source type: `handwritten`
- Rust target: `azurite-table/src/entity/edm_date_time.rs`
- Crate: `azurite-table`
- Module: `entity::edm_date_time`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IEdmType` | `impl IEdmType` | Trait impl for EDM type |

## Special handling
ISO 8601 datetime. Must preserve Z suffix exactly. Implements IEdmType trait with `to_json_property_value_pair()` and `to_json_property_type_pair()` for OData serialization. See aggregate record at `porting-db/src/table/entity/TableEntityTypes.md` for full type system details.
