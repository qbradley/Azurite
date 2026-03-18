# Porting Record — `src/table/entity/EdmBoolean.ts`

## File info
- Source path: `src/table/entity/EdmBoolean.ts`
- Source lines: `47`
- Source type: `handwritten`
- Rust target: `azurite-table/src/entity/edm_boolean.rs`
- Crate: `azurite-table`
- Module: `entity::edm_boolean`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IEdmType` | `impl IEdmType` | Trait impl for EDM type |

## Special handling
Boolean true/false. Implements IEdmType trait with `to_json_property_value_pair()` and `to_json_property_type_pair()` for OData serialization. See aggregate record at `porting-db/src/table/entity/TableEntityTypes.md` for full type system details.
