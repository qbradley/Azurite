# Porting Record — `src/blob/handlers/IPageBlobRangesManager.ts`

## File info
- Source path: `src/blob/handlers/IPageBlobRangesManager.ts`
- Source lines: `18`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/handlers/i_page_blob_ranges_manager.rs`
- Crate: `azurite-blob`
- Module: `handlers::i_page_blob_ranges_manager`
- Phase: `11.8`
- Status: `not_started`

## Exported API
### Default interface `IPageBlobRangesManager`
- `mergeRange(ranges, range): void`
- `clearRange(ranges, range): void`
- `cutRanges(ranges, range): PersistencyPageRange[]`
- `fillZeroRanges(ranges, range): PersistencyPageRange[]`

## Dependencies
- `PageRange` from generated blob models.
- `PersistencyPageRange` from `IBlobMetadataStore`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| mutable interface over page-range algorithms | trait or concrete helper struct API | The implementation mutates vectors in place for some methods and returns clones for others. |
| `PersistencyPageRange[]` | `Vec<PersistedPageRange>` | Keep ranges ordered and inclusive. |

## Special handling
- The interface intentionally mixes in-place mutators (`mergeRange`, `clearRange`) with pure-ish slicing/filling helpers (`cutRanges`, `fillZeroRanges`).
- `BlobHandler` and `PageBlobHandler` both depend on this contract; treat it as a shared cross-handler seam, not a page-only helper.

## Change propagation notes
- If TS expands this interface, port both consumers together so the Rust range helper surface stays mechanically parallel.
