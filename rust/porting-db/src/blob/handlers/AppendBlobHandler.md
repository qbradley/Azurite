# Porting Record — `src/blob/handlers/AppendBlobHandler.ts`

## File info
- Source path: `src/blob/handlers/AppendBlobHandler.ts`
- Source lines: `263`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/handlers/append_blob_handler.rs`
- Crate: `azurite-blob`
- Module: `handlers::append_blob_handler`
- Phase: `11.7`
- Status: `not_started`

## Exported API
### Default class `AppendBlobHandler extends BaseHandler implements IAppendBlobHandler`
- Methods:
  - `create()`
  - `appendBlock()`
  - `appendBlockFromUrl()` — unimplemented
  - `seal()`

## Dependencies
- `convertRawHeadersToMetadata()`, `getMD5FromStream()`, `newEtag()`.
- Append-blob constants: `MAX_APPEND_BLOB_BLOCK_SIZE`, `MAX_APPEND_BLOB_BLOCK_COUNT`, `HeaderConstants`.
- `IBlobMetadataStore` create/download/append/seal APIs.
- `getTagsFromString()` for header-based tag ingestion.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| append blob state as normal blob model + `committedBlocksInOrder` | same persisted blob struct with append-specific fields | Do not split append blobs into a separate persistence model. |
| streamed append with optional MD5 validation | write-and-validate helper | Preserve the current post-write MD5 verification flow. |

## Special handling
- `AppendBlobHandler.ts:33-38` permits nonzero `Content-Length` on create when `loose` mode is enabled; strict mode rejects it.
- `AppendBlobHandler.ts:72-77` initializes append blobs with `isSealed: false` and an empty committed-block list.
- `AppendBlobHandler.ts:114-123` rejects zero-length append blocks as `InvalidHeaderValue(content-length=0)` rather than a generic invalid operation.
- `AppendBlobHandler.ts:125-141` enforces the 50,000 committed-block limit in the handler before writing anything.
- `AppendBlobHandler.ts:125-132`, `187-201` do not run an explicit lease validator in the handler; lease/append-position enforcement is delegated to store methods.
- `AppendBlobHandler.ts:157-182` validates only the standard `content-md5` header and returns `Md5Mismatch` with both provided and calculated base64 digests.
- `AppendBlobHandler.ts:185-215` returns the pre-append content length as `blobAppendOffset` and increments committed block count in the response.
- `AppendBlobHandler.ts:221-227` leaves append-from-URL unsupported.
- `AppendBlobHandler.ts:241-248` calls `sealBlob()` with `snapshot = undefined` and forwards the whole `options` bag.

## Change propagation notes
- If TS later adds handler-side lease validation here, keep that change visible because append/blob/page handlers do not currently validate leases uniformly.
- `loose` mode is compatibility-sensitive for append-blob create; do not normalize it away in Rust.
