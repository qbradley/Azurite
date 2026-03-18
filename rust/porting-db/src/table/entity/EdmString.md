# Porting Record — `src/table/entity/EdmString.ts`

## File info
- Source path: `src/table/entity/EdmString.ts`
- Source lines: `30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/entity/edm_string.rs`
- Crate: `azurite-table`
- Module: `entity::edm_string`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IEdmType` | `impl IEdmType` | Trait impl for EDM type |

## Special handling
Default/implicit type — no annotation at Minimal level. Implements IEdmType trait with `to_json_property_value_pair()` and `to_json_property_type_pair()` for OData serialization. See aggregate record at `porting-db/src/table/entity/TableEntityTypes.md` for full type system details.
