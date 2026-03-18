# Porting Record — `src/table/entity/IEdmType.ts`

## File info
- Source path: `src/table/entity/IEdmType.ts`
- Source lines: `~50`
- Source type: `handwritten`
- Rust target: `azurite-table/src/entity/i_edm_type.rs`
- Crate: `azurite-table`
- Module: `entity::i_edm_type`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `interface IEdmType` | `trait IEdmType` | EDM type interface |
| `enum EdmType` | `enum EdmType` | Type discriminant |

## Special handling
Core EDM type trait and enum. All EDM types implement this trait. `get_edm_type()` dispatch function maps type strings to enum variants.
