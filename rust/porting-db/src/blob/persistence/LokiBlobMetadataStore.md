# Porting Record — `src/blob/persistence/LokiBlobMetadataStore.ts`

## File info
- Source path: `src/blob/persistence/LokiBlobMetadataStore.ts`
- Source lines: `3565`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/loki_blob_metadata_store.rs`
- Crate: `azurite-blob`
- Module: `persistence::loki_blob_metadata_store`
- Phase: `10.7`
- Status: `in_progress` (1733 lines translated, 35 of 51 methods implemented, compilation errors present)

## Exported API
### Default class `LokiBlobMetadataStore`
- Implements `IBlobMetadataStore` and `IGCExtentProvider`.
- Constructor: `new LokiBlobMetadataStore(lokiDBPath: string, inMemory: boolean)`
- Lifecycle: `isInitialized()`, `isClosed()`, `init()`, `close()`, `clean()`
- GC: `iteratorExtents()`
- Implements every `IBlobMetadataStore` method group: service properties, containers, container leases, blob listing/filtering, blob CRUD/copy/tier, blob leases, block/page/append operations, tags, and seal-blob.
- Private helper families: `restoreUint8Array()`, `escapeRegex()`, overloaded `getContainerWithLeaseUpdated()`, `getContainer()`, `getBlobWithLeaseUpdated()`, `getBlob()`, plus tier parsing.

## Dependencies
- `lokijs`, `fs.stat`, and `uuid/v4` for the concrete metadata backend and ID generation.
- Common helpers: `convertDateTimeStringMsTo7Digital`, `rimrafAsync`, `newEtag`.
- Condition validators from `../conditions/*`.
- All lease adapters/validators/syncers plus `LeaseFactory` from Phase 8.
- `PageBlobRangesManager`, `BlobReferredExtentsAsyncIterator`, `PageWithDelimiter`, `FilterBlobPage`, and the query interpreter helpers.
- `getBlobTagsCount`, `getTagsFromString`, and `toBlobTags` tag utilities.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `Loki` database + collections | repository layer over in-memory/fs-backed store | If Rust swaps storage engines, preserve collection boundaries, indices, and pagination semantics. |
| mutable document updates | owned structs inside locks/transactions | TS mutates Loki documents in place and then calls `coll.update(doc)`. |
| `Uint8Array` round-trip restoration | binary field adapter | Loki JSON persistence loses typed arrays, so reads must restore them before use. |
| Phase 8 lease subsystem | shared state-machine module | This store is the main consumer of the lease adapters, factory, validators, and syncers. |

## Special handling
- `LokiBlobMetadataStore.ts:77-93` documents four collections: service properties, containers, blobs, and uncommitted blocks, each with its own uniqueness/indexing assumptions.
- `LokiBlobMetadataStore.ts:111-191` chooses `memory` or `fs` persistence at construction, enables autosave every 5000 ms for fs mode, and explicitly comments that Loki operations are synchronous so no async lock is used.
- `LokiBlobMetadataStore.ts:328-345` works around a Loki quirk where `$regex` ignores `$gt` by issuing a second `.find({ name: { $gt: marker } })` during container listing.
- `LokiBlobMetadataStore.ts:827-899` wires tag filtering through `generateQueryBlobWithTagsWhereFunction()`, converts each blob to a `{ name, containerName, tags }` view, and then rewrites successful matches back into narrowed `blobTagSet` values via `toBlobTags()`.
- `LokiBlobMetadataStore.ts:901-970` and `973-1009` rely on `PageWithDelimiter` / manual `maxResults + 1` over-fetching to implement continuation markers.
- `LokiBlobMetadataStore.ts:1531-1765` shows the standard blob-lease flow: fetch via `getBlobWithLeaseUpdated()`, validate write conditions, reject snapshots, run the Phase 8 state machine, sync back into the doc, then `coll.update(doc)`.
- `LokiBlobMetadataStore.ts:3203-3226` and `3337-3402` are the lazy timer checkpoints for containers and blobs: each call re-runs `LeaseFactory.createLeaseState(...).sync(...)` before returning the document.
- `LokiBlobMetadataStore.ts:3385-3395` forcibly clears snapshot lease fields to `Available` / `Unlocked` even though the TODO comments say snapshot lease state/status should really be `undefined`.
- `LokiBlobMetadataStore.ts:3419-3448` ignores `modifiedAccessConditions` in `setBlobTag()` even though the interface accepts them.
- `LokiBlobMetadataStore.ts:996-1000`, `3379-3383`, and helper `restoreUint8Array()` are compatibility glue for persisted `contentMD5` binary fields.

## Change propagation notes
- If Aragorn replaces Loki with another backend, preserve lazy lease updates, manual marker semantics, and the document-shape quirks before chasing storage-engine idioms.
- Tag-query behavior spans this file, `QueryInterpreter.ts`, and the query-node AST; update them as a unit whenever TS changes filtering semantics.
