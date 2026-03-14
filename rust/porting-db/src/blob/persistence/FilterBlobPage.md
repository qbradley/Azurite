# Porting Record — `src/blob/persistence/FilterBlobPage.ts`

## File info
- Source path: `src/blob/persistence/FilterBlobPage.ts`
- Source lines: `128`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/filter_blob_page.rs`
- Crate: `azurite-blob`
- Module: `persistence::filter_blob_page`
- Phase: `10.9`
- Status: `not_started`

## Exported API
### Default class `FilterBlobPage<FilterBlobType>`
- Constructor: `new FilterBlobPage(maxResults: number)`
- Public methods: `reset()`, `fill(reader, namer)`
- Internal helpers: `updateFull()`, `addItem()`, `add()`, `processList()`
- Public state exposed for tests/callers: `filterBlobItems`, `latestMarker`

## Dependencies
- Generic over the item shape; no imports beyond language/runtime constructs.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| generic page buffer | generic Rust struct over `T` | Used for filtered blob listings without delimiter/prefix squashing. |
| reader callback | async closure/fn taking `offset` | Pagination repeatedly pulls sorted batches until the page fills or the source exhausts. |

## Special handling
- `FilterBlobPage.ts:16-20` tracks two booleans: `isFull` means only duplicate prefixes could still fit (not used here), while `isExhausted` means nothing more should be added.
- `FilterBlobPage.ts:67-80` requires sorted input and throws a generic `Error` if `add()` sees a name lower than `latestMarker`.
- `FilterBlobPage.ts:108-126` keeps calling `reader(offset)` until either a batch adds fewer than `maxResults` items or the data source ends. The continuation token is `latestMarker` only when the page filled before consuming the whole batch.

## Change propagation notes
- If TS changes continuation-token semantics, update this helper and `LokiBlobMetadataStore.filterBlobs()` together.
- Preserve the sorted-input assumption; changing it would affect every caller and test that relies on marker monotonicity.
