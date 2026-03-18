# Porting Record — `src/table/entity/EdmDouble.ts`

## File info
- Source path: `src/table/entity/EdmDouble.ts`
- Source lines: `88`
- Source type: `handwritten`
- Rust target: `azurite-table/src/entity/edm_double.rs`
- Crate: `azurite-table`
- Module: `entity::edm_double`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IEdmType` | `impl IEdmType` | Trait impl for EDM type |

## Special handling
IEEE 754 double. Special handling for NaN/Infinity string representation. Implements IEdmType trait with `to_json_property_value_pair()` and `to_json_property_type_pair()` for OData serialization. See aggregate record at `porting-db/src/table/entity/TableEntityTypes.md` for full type system details.
