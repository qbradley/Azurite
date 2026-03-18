# Porting Record — `src/queue/generated/artifacts/models.ts`

## File info
- Source path: `src/queue/generated/artifacts/models.ts`
- Source lines: `1674`
- Source type: `generated (autorest)`
- Rust target: `azurite-queue/src/generated/artifacts/models.rs`
- Crate: `azurite-queue`
- Module: `generated::artifacts::models`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `interface` | `struct` | Model types |
| `Mapper` | `serialization fns` | Ser/deser logic |
| `enum` | `enum` | Operation variants |

## Special handling
Data models (AccessPolicy, QueueItem, QueueMessage, ListQueuesSegmentResponse, etc.). Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/artifacts/models.md` for detailed fidelity notes.
