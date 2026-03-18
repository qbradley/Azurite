# Porting Record — `src/table/entity/EdmInt64.ts`

## File info
- Source path: `src/table/entity/EdmInt64.ts`
- Source lines: `64`
- Source type: `handwritten`
- Rust target: `azurite-table/src/entity/edm_int64.rs`
- Crate: `azurite-table`
- Module: `entity::edm_int64`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IEdmType` | `impl IEdmType` | Trait impl for EDM type |

## Special handling
64-bit signed integer. TS uses BigNumber for precision; Rust uses i64. Implements IEdmType trait with `to_json_property_value_pair()` and `to_json_property_type_pair()` for OData serialization. See aggregate record at `porting-db/src/table/entity/TableEntityTypes.md` for full type system details.
