# Porting Record — `src/table/entity/EdmInt32.ts`

## File info
- Source path: `src/table/entity/EdmInt32.ts`
- Source lines: `54`
- Source type: `handwritten`
- Rust target: `azurite-table/src/entity/edm_int32.rs`
- Crate: `azurite-table`
- Module: `entity::edm_int32`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IEdmType` | `impl IEdmType` | Trait impl for EDM type |

## Special handling
32-bit signed integer. Implements IEdmType trait with `to_json_property_value_pair()` and `to_json_property_type_pair()` for OData serialization. See aggregate record at `porting-db/src/table/entity/TableEntityTypes.md` for full type system details.
