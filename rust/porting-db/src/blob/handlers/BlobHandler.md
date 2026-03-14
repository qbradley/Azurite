# Porting Record — `src/blob/handlers/BlobHandler.ts`

## File info
- Source path: `src/blob/handlers/BlobHandler.ts`
- Source lines: `1350`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/handlers/blob_handler.rs`
- Crate: `azurite-blob`
- Module: `handlers::blob_handler`
- Phase: `11.4`
- Status: `not_started`

## Exported API
### Default class `BlobHandler extends BaseHandler implements IBlobHandler`
- Constructor injects shared stores/logger/loose plus `rangesManager: IPageBlobRangesManager`.
- Public methods:
  - data/property ops: `download()`, `getProperties()`, `delete()`, `setHTTPHeaders()`, `setMetadata()`, `setTier()`
  - unsupported placeholders: `undelete()`, `setExpiry()`, `setImmutabilityPolicy()`, `deleteImmutabilityPolicy()`, `setLegalHold()`, `query()`
  - lease ops: `acquireLease()`, `releaseLease()`, `renewLease()`, `changeLease()`, `breakLease()`
  - snapshot/copy ops: `createSnapshot()`, `startCopyFromURL()`, `abortCopyFromURL()`, `copyFromURL()`
  - tag/account info ops: `getTags()`, `setTags()`, `getAccountInfo()`, `getAccountInfoWithHead()`
- Private helpers:
  - `validateCopySource()`
  - `downloadBlockBlobOrAppendBlob()`
  - `downloadPageBlob()`
  - `NewUriFromCopySource()`

## Dependencies
- `IBlobMetadataStore` and `IExtentStore` drive almost every method.
- `IPageBlobRangesManager` is required for page-blob hole filling and range trimming.
- `axios`, `URLBuilder`, WHATWG `URL`, `parseXML()`, `extractStoragePartsFromPath()`.
- Phase 8 lease semantics and Phase 9 conditional semantics mostly live in the metadata store APIs.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| huge operation multiplexer | one Rust struct with many async fns plus a few private helpers | Keep method boundaries aligned with generated handler interface names. |
| `NodeJS.ReadableStream` response bodies | async stream body adapters | Download methods rely on deferred stream creation and sometimes re-open streams to recompute MD5. |
| page range arithmetic via injected manager | separate helper trait/struct | Blob/page handlers intentionally share the same range engine. |
| copy-source validation via outbound HTTP | explicit internal client helper | This is compatibility behavior, not an implementation detail to remove. |

## Special handling
- `BlobHandler.ts:73-95` dispatches downloads by stored blob type; archived blobs fail early with `getBlobArchived()`.
- `BlobHandler.ts:124-161` folds `comp=metadata` into `getProperties()` instead of using a separate handler and overlays response-header query params (`rscc`, `rscd`, `rsce`, `rscl`, `rsct`) onto stored properties.
- `BlobHandler.ts:245-277` reroutes `setHTTPHeaders()` into `updateSequenceNumber()` when `x-ms-sequence-number-action` is present; this workaround is part of observable request routing.
- `BlobHandler.ts:214-223`, `293-313`, `1261-1265` leave several APIs unimplemented; Rust should mirror that status, not invent behavior.
- `BlobHandler.ts:380-385` rejects lease acquisition for snapshots with the explicit message `A lease cannot be granted for a blob snapshot`.
- `BlobHandler.ts:594-608` only forwards snapshot metadata when `options.metadata` is non-empty; `{}` and missing metadata both become `undefined`.
- `BlobHandler.ts:644-664` validates copy sources when the source account differs from the destination account **or** when the URL carries `sig`; `copyFromURL()` is slightly looser and only validates when the source account differs (`BlobHandler.ts:860-862`).
- `BlobHandler.ts:701-775` validates cross-account copy access by issuing a GET metadata request, not HEAD, so that XML error details are available. It only parses error text when `content-type` equals exactly `application/xml`.
- `BlobHandler.ts:706-719` requires the source URL host to match the incoming `Host` header exactly; cross-instance copy is rejected as `CannotVerifyCopySource`.
- `BlobHandler.ts:787-821` `abortCopyFromURL()` only validates copy state and returns `204`; it does not call the metadata store to mutate anything.
- `BlobHandler.ts:864-867` forbids `x-ms-copy-source-tag-option=COPY` together with explicit destination tags.
- `BlobHandler.ts:889-899` insists synchronous `copyFromURL()` ends in `CopyStatusType.Success`; any other store result becomes `UnexpectedSyncCopyStatus`.
- `BlobHandler.ts:1007-1019` deliberately ignores malformed block/append range headers per RFC 9110 instead of failing the request.
- `BlobHandler.ts:1024-1038`, `1152-1166` clamp end-of-range past EOF, but preserve the empty-blob edge case that still throws for nonzero explicit end values.
- `BlobHandler.ts:1087-1094`, `1215-1222` compute MD5 on demand only for bodies up to 4 MiB and reopen the body stream afterward.
- `BlobHandler.ts:1114` and `1250` only return per-range `contentMD5` when a range response is being sent **and** the caller asked for `x-ms-range-get-content-md5`.
- `BlobHandler.ts:1192-1199` uses `fillZeroRanges()` so page-blob holes are materialized as zero extents during download.
- `BlobHandler.ts:1311-1323` manually reads the `snapshot` query parameter in `setTags()` because swagger does not model it even though Azurite supports it.
- `BlobHandler.ts:1336-1348` wraps invalid copy-source URLs as `InvalidHeaderValue` for `x-ms-copy-source`.

## Change propagation notes
- This file is the main consumer of Phase 8 leases, Phase 9 conditions, Phase 10 persistence, and Phase 11 page-range/batch helpers. Port and update those units together.
- Any future TS copy-path change must be audited in `startCopyFromURL()`, `copyFromURL()`, and `validateCopySource()` simultaneously; the validation conditions are intentionally asymmetric today.
