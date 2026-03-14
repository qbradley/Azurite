# Porting Record — `src/blob/handlers/ContainerHandler.ts`

## File info
- Source path: `src/blob/handlers/ContainerHandler.ts`
- Source lines: `865`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/handlers/container_handler.rs`
- Crate: `azurite-blob`
- Module: `handlers::container_handler`
- Phase: `11.3`
- Status: `not_started`

## Exported API
### Default class `ContainerHandler extends BaseHandler implements IContainerHandler`
- Constructor injects `accountDataStore`, `oauth`, shared stores/logger/loose, and optional `disableProductStyle`.
- Methods:
  - container CRUD: `create()`, `getProperties()`, `getPropertiesWithHead()`, `delete()`, `setMetadata()`
  - ACL / public access: `getAccessPolicy()`, `setAccessPolicy()`
  - unsupported: `restore()`
  - batch/tag filtering: `submitBatch()`, `filterBlobs()`
  - lease methods: `acquireLease()`, `releaseLease()`, `renewLease()`, `breakLease()`, `changeLease()`
  - listing: `listBlobFlatSegment()`, `listBlobHierarchySegment()`
  - account info: `getAccountInfo()`, `getAccountInfoWithHead()`

## Dependencies
- `BlobBatchHandler` for container-scoped batch requests.
- `convertRawHeadersToMetadata()`, `newEtag()`, `getBlobTagsCount()`, `removeQuotationFromListBlobEtag()`.
- `IBlobMetadataStore` container/blob listing and lease APIs.
- Lease behavior depends on Phase 8 store logic rather than explicit handler-side state machines.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| large handler class with many response builders | struct with one async fn per REST operation | Keep the one-method-per-operation shape because generated dispatch depends on it. |
| metadata via raw header preservation | ordered/raw-header adapter | Metadata key case is intentionally preserved from raw headers. |
| `any` array/object hybrid in `getAccessPolicy()` | explicit response shim or hand serializer | This is a real generator-compatibility quirk, not dead code. |

## Special handling
- `ContainerHandler.ts:65-68`, `206-209` preserve metadata key case by re-reading raw headers instead of trusting deserialized metadata maps.
- `ContainerHandler.ts:165-170` documents a known fidelity gap: delete removes metadata immediately and does not asynchronously clean extent bytes first.
- `ContainerHandler.ts:257-279` builds `getAccessPolicy()` as an `any = []` value that is both an array of `SignedIdentifier` and an object carrying headers/status because generated XML serialization is wrong.
- `ContainerHandler.ts:274-278` captures the current generator XML bug: Azurite wants `<SignedIdentifiers>` but currently emits a `<parsedResponse>` wrapper.
- `ContainerHandler.ts:327-332` leaves `restore()` unimplemented.
- `ContainerHandler.ts:334-366` wraps `BlobBatchHandler`, enforces the current container path prefix, and returns `202` with `multipart/mixed; boundary=` using the request boundary value.
- `ContainerHandler.ts:381-388`, `644-671`, `748-777` mutate `options.marker` / `options.prefix` defaults in place; response payloads expose those mutated values rather than the original `undefined`s.
- `ContainerHandler.ts:650-664` and `755-770` treat `include` values case-insensitively for snapshots, uncommitted blobs, tags, and metadata.
- `ContainerHandler.ts:699-713` strips quotes from listed ETags and converts `accessTierInferred === true` to `true`, otherwise `undefined`.
- `ContainerHandler.ts:737` keeps a TODO about stale lease attributes in hierarchy listing; the handler currently trusts store output as-is.
- `ContainerHandler.ts:807-821` mutates `item.deleted` before spreading the blob item in hierarchy listing.

## Change propagation notes
- Container list/filter behavior is tightly coupled to Phase 10 store pagination helpers (`PageWithDelimiter`, `FilterBlobPage`, tag query interpreter). Update those units together.
- If the generated XML serializer is regenerated, revisit `getAccessPolicy()` first; that method currently exists around a generator bug, not around Azure semantics alone.
