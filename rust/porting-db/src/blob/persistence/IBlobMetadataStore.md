# Porting Record — `src/blob/persistence/IBlobMetadataStore.ts`

## File info
- Source path: `src/blob/persistence/IBlobMetadataStore.ts`
- Source lines: `1165`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/i_blob_metadata_store.rs`
- Crate: `azurite-blob`
- Module: `persistence::i_blob_metadata_store`
- Phase: `10.1`
- Status: `not_started`

## Exported API
### Exported models and aliases
- `IExtentChunk { id, offset, count }`
- `ZERO_EXTENT_ID = "*ZERO*"`
- `ServicePropertiesModel`
- `ContainerModel`, `IContainerMetadata`, container lease/access-policy response aliases
- `PersistencyPageRange`, `BlobModel`, `BlobPrefixModel`, `GetBlobPropertiesRes`, `FilterBlobModel`
- Blob lease response aliases, `CreateSnapshotResponse`, `BlobId`, `GetPageRangeResponse`
- `PersistencyBlockModel`, `BlockModel`

### Default interface `IBlobMetadataStore extends IGCExtentProvider, IDataStore, ICleaner`
- Service: `setServiceProperties()`, `getServiceProperties()`
- Containers: `listContainers()`, `createContainer()`, `getContainerProperties()`, `deleteContainer()`, `setContainerMetadata()`, `getContainerACL()`, `setContainerACL()`, `checkContainerExist()`
- Container leases: `acquireContainerLease()`, `releaseContainerLease()`, `renewContainerLease()`, `breakContainerLease()`, `changeContainerLease()`
- Blob listing/filtering: `listBlobs()`, `listAllBlobs()`, `filterBlobs()`
- Blob CRUD and copy/tier: `createBlob()`, `createSnapshot()`, `downloadBlob()`, `getBlobProperties()`, `deleteBlob()`, `setBlobHTTPHeaders()`, `setBlobMetadata()`, `checkBlobExist()`, `getBlobType()`, `startCopyFromURL()`, `copyFromURL()`, `setTier()`
- Blob leases: `acquireBlobLease()`, `releaseBlobLease()`, `renewBlobLease()`, `changeBlobLease()`, `breakBlobLease()`
- Block/page/append: `stageBlock()`, `appendBlock()`, `commitBlockList()`, `getBlockList()`, `uploadPages()`, `clearRange()`, `getPageRanges()`, `resizePageBlob()`, `updateSequenceNumber()`, `listUncommittedBlockPersistencyChunks()`
- Tags and sealing: `setBlobTag()`, `getBlobTag()`, `sealBlob()`

## Dependencies
- `@azure/storage-blob.BlobTags` plus generated `Models.*` types from `../generated/artifacts/models`.
- `../../common/ICleaner`, `IDataStore`, and `IGCExtentProvider`.
- `../generated/Context` for request-scoped validation and timestamps.
- Implemented by `LokiBlobMetadataStore` (Phase 10.7) and `SqlBlobMetadataStore` (later phase).

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| TypeScript intersection aliases | flattened Rust structs per strategy | Keep the distinct exported model names (`BlobModel`, `ContainerModel`, `BlockModel`, etc.) even if implemented as composed structs. |
| `IExtentChunk` | `struct ExtentChunk { id, offset, count }` | Represents a pointer into external extent persistence, not inline blob bytes. |
| `Promise<[T[], string | undefined]>` | `Result<(Vec<T>, Option<String>), StorageError>` | Preserve tuple pagination across listing/filtering methods. |
| mixed `BlobTags` imports | explicit adapter layer | TS already mixes SDK `BlobTags` and generated `Models.BlobTags`; keep that asymmetry visible in Rust. |

## Special handling
- `IBlobMetadataStore.ts:16-22` defines `IExtentChunk` plus `ZERO_EXTENT_ID = "*ZERO*"`, which is already known to leak into common-layer extent code.
- `IBlobMetadataStore.ts:42-205` builds its main models by intersecting generated swagger shapes with emulator-only fields such as `accountName`, lease timing, persistency pointers, and committed/page-range arrays.
- `IBlobMetadataStore.ts:157-169` has a response alias quirk: `ReleaseBlobLeaseResponse` points at `Models.ContainerProperties`, not blob properties.
- `IBlobMetadataStore.ts:297-304` documents `deleteContainer()` as a two-phase GC-aware delete that marks the container as deleting before eventual cleanup.
- `IBlobMetadataStore.ts:501-515`, `809-816`, and `1091-1094` show that not every method accepts `Context`; `listAllBlobs()`, `getBlobType()`, and `listUncommittedBlockPersistencyChunks()` are notable exceptions.
- `IBlobMetadataStore.ts:1110-1142` mixes `Models.BlobTags` on write with SDK `BlobTags` on read, so a literal one-type simplification would hide an existing TS asymmetry.

## Change propagation notes
- If any model field changes upstream, audit the interface, `LokiBlobMetadataStore`, query interpreter helpers, and extent iterators together.
- Treat the response alias quirks (`ReleaseBlobLeaseResponse`, mixed tag types) as compatibility-sensitive until Quetzal approves cleanup.
