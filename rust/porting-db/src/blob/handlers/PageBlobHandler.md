# Porting Record — `src/blob/handlers/PageBlobHandler.ts`

## File info
- Source path: `src/blob/handlers/PageBlobHandler.ts`
- Source lines: `495`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/handlers/page_blob_handler.rs`
- Crate: `azurite-blob`
- Module: `handlers::page_blob_handler`
- Phase: `11.6`
- Status: `not_started`

## Exported API
### Default class `PageBlobHandler extends BaseHandler implements IPageBlobHandler`
- Constructor injects shared stores/logger/loose plus `rangesManager`.
- Methods:
  - `uploadPagesFromURL()` — unimplemented
  - `create()`
  - `uploadPages()`
  - `clearPages()`
  - `getPageRanges()`
  - `getPageRangesDiff()` — unimplemented
  - `resize()`
  - `updateSequenceNumber()`
  - `copyIncremental()` — unimplemented

## Dependencies
- `BlobLeaseAdapter` + `BlobWriteLeaseValidator` for explicit lease validation on page writes.
- `deserializePageBlobRangeHeader()` and `getTagsFromString()`.
- `IPageBlobRangesManager` for slicing/filling persisted ranges.
- `IBlobMetadataStore` page-blob store operations.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| explicit 512-byte boundary validation | helper returning `(start, end)` or storage error | Boundary enforcement is observable and reused in multiple methods. |
| page-range manager injection | shared helper object/trait | Blob and page handlers must use the same range semantics. |
| handler-side lease validation before store call | explicit pre-store validator invocation | Not every method validates leases in the same layer. |

## Special handling
- `PageBlobHandler.ts:63-81` rejects any explicit tier on create and enforces `contentLength === 0` plus `x-ms-content-length` alignment to 512-byte boundaries.
- `PageBlobHandler.ts:124-126` defaults missing page-blob sequence numbers to `0`.
- `PageBlobHandler.ts:199-203` explicitly validates write leases in `uploadPages()` before range parsing, while `clearPages()` relies on store-layer validation instead.
- `PageBlobHandler.ts:206-224`, `297-314`, `365-377` parse range headers through `deserializePageBlobRangeHeader()` and then apply additional start-of-file bounds checks.
- `PageBlobHandler.ts:218-220` requires the parsed page range length to match `contentLength` exactly for uploads.
- `PageBlobHandler.ts:379-387` trims `getPageRanges()` output through `rangesManager.cutRanges()` rather than returning the whole stored range list.
- `PageBlobHandler.ts:403-408`, `488-493`, `40-49` leave diff copy/upload-from-URL functionality unimplemented.
- `PageBlobHandler.ts:421-436` `resize()` only enforces 512-byte alignment at the handler layer and otherwise delegates type/condition handling to the store.

## Change propagation notes
- Keep page-range math in `PageBlobRangesManager` instead of duplicating it here; future TS fixes are likely to land there first.
- If TS adds lease checks to `clearPages()` or `resize()`, preserve the layer where the validation occurs (handler vs store) because request-failure timing can shift.
