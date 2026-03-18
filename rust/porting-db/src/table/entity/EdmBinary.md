# Porting Record — `src/table/entity/EdmBinary.ts`

## File info
- Source path: `src/table/entity/EdmBinary.ts`
- Source lines: `64`
- Source type: `handwritten`
- Rust target: `azurite-table/src/entity/edm_binary.rs`
- Crate: `azurite-table`
- Module: `entity::edm_binary`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IEdmType` | `impl IEdmType` | Trait impl for EDM type |

## Special handling
Base64-encoded binary data. Implements IEdmType trait with `to_json_property_value_pair()` and `to_json_property_type_pair()` for OData serialization. See aggregate record at `porting-db/src/table/entity/TableEntityTypes.md` for full type system details.
