# Porting Record — `src/queue/persistence/QueueReferredExtentsAsyncIterator.ts`

## File info
- Source path: `src/queue/persistence/QueueReferredExtentsAsyncIterator.ts`
- Source lines: `38`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/persistence/queue_referred_extents_async_iterator.rs`
- Crate: `azurite-queue`
- Module: `persistence::queue_referred_extents_async_iterator`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `AsyncIterator` | `Stream / async iterator` | Async iteration over extent refs |
| `IExtentChunk` | `ExtentChunk struct` | Extent reference record |

## Special handling
Async iterator yielding extent chunks referenced by queue messages. Used by GC to determine which extents are still in use. Follows same pattern as blob equivalent. See `porting-db/src/blob/persistence/BlobReferredExtentsAsyncIterator.md` for detailed fidelity notes.
