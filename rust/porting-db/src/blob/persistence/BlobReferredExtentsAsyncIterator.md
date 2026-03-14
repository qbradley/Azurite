# Porting Record — `src/blob/persistence/BlobReferredExtentsAsyncIterator.ts`

## File info
- Source path: `src/blob/persistence/BlobReferredExtentsAsyncIterator.ts`
- Source lines: `110`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/blob_referred_extents_async_iterator.rs`
- Crate: `azurite-blob`
- Module: `persistence::blob_referred_extents_async_iterator`
- Phase: `10.8`
- Status: `not_started`

## Exported API
### Default class `BlobReferredExtentsAsyncIterator`
- Implements `AsyncIterator<string[]>`.
- Constructor: `new BlobReferredExtentsAsyncIterator(blobMetadataStore: IBlobMetadataStore, logger?: ILogger)`
- Method: `next(): Promise<IteratorResult<string[]>>`
- Internal enum `State { LISTING_EXTENTS_IN_BLOBS, LISTING_EXTENTS_IN_BLOCKS, DONE }`

## Dependencies
- `../../common/ILogger`, `Logger`, and `NoLoggerStrategy`.
- `./IBlobMetadataStore`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `AsyncIterator<string[]>` | `Stream<Item = Vec<String>>` | Each `next()` yields a batch of extent IDs rather than single IDs. |
| state enum + markers | explicit iterator state machine | Blob listing and uncommitted-block listing use separate continuation markers. |

## Special handling
- `BlobReferredExtentsAsyncIterator.ts:21-29` defaults the logger to `new Logger(new NoLoggerStrategy())`, so there is still a real logger object even when output is suppressed.
- `BlobReferredExtentsAsyncIterator.ts:31-80` first walks committed blobs via `listAllBlobs(undefined, marker, true, true)`, so snapshots and uncommitted blobs are both included in extent discovery.
- `BlobReferredExtentsAsyncIterator.ts:67-75` extracts extent IDs from committed blocks, page ranges, and the blob-level persistency pointer for each blob.
- `BlobReferredExtentsAsyncIterator.ts:81-102` then switches to uncommitted-block extents via `listUncommittedBlockPersistencyChunks()` before finally returning `{ done: true, value: [] }`.

## Change propagation notes
- If `listAllBlobs()` marker semantics change, this iterator and GC callers must be updated in lockstep.
- Keep the two-phase iteration order (blobs first, blocks second) unless upstream TS changes it.
