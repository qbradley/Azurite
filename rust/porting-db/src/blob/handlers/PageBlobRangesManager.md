# Porting Record — `src/blob/handlers/PageBlobRangesManager.ts`

## File info
- Source path: `src/blob/handlers/PageBlobRangesManager.ts`
- Source lines: `481`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/handlers/page_blob_ranges_manager.rs`
- Crate: `azurite-blob`
- Module: `handlers::page_blob_ranges_manager`
- Phase: `11.9`
- Status: `not_started`

## Exported API
### Default class `PageBlobRangesManager implements IPageBlobRangesManager`
- Public methods:
  - `mergeRange()`
  - `clearRange()`
  - `cutRanges()`
  - `fillZeroRanges()`
  - `selectImpactedRanges()`
  - `locateFirstImpactedRange()`
  - `locateLastImpactedRange()`
  - `positionInRange()`

## Dependencies
- `PersistencyPageRange`, `ZERO_EXTENT_ID` from `IBlobMetadataStore`.
- `PageRange` from generated models.
- Consumed by both `BlobHandler` and `PageBlobHandler`, and indirectly by Phase 10 store logic.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| ordered mutable array of inclusive ranges | `Vec<PersistedPageRange>` kept sorted by `start` | Binary-search helpers assume sorted non-overlapping ranges. |
| `{ id, offset, count }` persistency slices | small value struct | Split operations adjust offsets/counts precisely; preserve integer arithmetic. |
| zero-fill sentinel | shared `ZERO_EXTENT_ID` constant | Download paths rely on this exact sentinel to synthesize holes. |

## Special handling
- `PageBlobRangesManager.ts:8-49` chooses the “split first/last impacted ranges, leave compaction to future GC” strategy specifically to keep page-range diff fidelity against snapshots.
- `PageBlobRangesManager.ts:51-114` `mergeRange()` replaces overlapped ranges with up to three segments: left remainder, new range, right remainder.
- `PageBlobRangesManager.ts:117-169` `clearRange()` uses the same split logic but omits the middle segment entirely.
- `PageBlobRangesManager.ts:180-238` `cutRanges()` returns cloned impacted slices trimmed to the requested boundaries without mutating the source array.
- `PageBlobRangesManager.ts:241-312` `fillZeroRanges()` first cuts to the requested window, then inserts synthetic `ZERO_EXTENT_ID` ranges before, between, and after existing data.
- `PageBlobRangesManager.ts:333-358` returns `[Infinity, -1]` for an empty range set and otherwise uses the recursive binary-search helpers to bracket impacted ranges.
- `PageBlobRangesManager.ts:337-341` throws `RangeError` when `start > end` or `start < 0`.
- `PageBlobRangesManager.ts:379` and `436` contain a no-op `searchEnd` assignment (`searchEnd = searchEnd > ranges.length ? searchEnd : searchEnd;`). It looks like a clamp bug, but current callers pass valid lengths so the typo is latent. Preserve/document rather than “fixing” silently.
- `PageBlobRangesManager.ts:44-48` carries open TODOs for resize cleanup, post-resize downloads, clear-range strategy, range-merging GC, and unreferred extent GC.

## Change propagation notes
- This file is the canonical page-range arithmetic source for the Rust port. If TS changes any split/merge rule, Aragorn must update blob download, page-blob ops, and Phase 10 persistence code together.
- Do not replace this with an interval-tree or other “better” structure unless the public behavior is proven identical; fidelity matters more than elegance here.
