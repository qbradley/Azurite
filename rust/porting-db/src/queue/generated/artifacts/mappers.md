# Porting Record — `src/queue/generated/artifacts/mappers.ts`

## File info
- Source path: `src/queue/generated/artifacts/mappers.ts`
- Source lines: `1387`
- Source type: `generated (autorest)`
- Rust target: `azurite-queue/src/generated/artifacts/mappers.rs`
- Crate: `azurite-queue`
- Module: `generated::artifacts::mappers`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `interface` | `struct` | Model types |
| `Mapper` | `serialization fns` | Ser/deser logic |
| `enum` | `enum` | Operation variants |

## Special handling
Serialization/deserialization mappers for queue API models. Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/artifacts/mappers.md` for detailed fidelity notes.
