# Porting Record — `src/table/entity/EdmGuid.ts`

## File info
- Source path: `src/table/entity/EdmGuid.ts`
- Source lines: `104`
- Source type: `handwritten`
- Rust target: `azurite-table/src/entity/edm_guid.rs`
- Crate: `azurite-table`
- Module: `entity::edm_guid`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IEdmType` | `impl IEdmType` | Trait impl for EDM type |

## Special handling
UUID in RFC 4122 format. Case-insensitive parse, lowercase output. Implements IEdmType trait with `to_json_property_value_pair()` and `to_json_property_type_pair()` for OData serialization. See aggregate record at `porting-db/src/table/entity/TableEntityTypes.md` for full type system details.
