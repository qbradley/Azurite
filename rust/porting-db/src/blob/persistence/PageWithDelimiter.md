# Porting Record — `src/blob/persistence/PageWithDelimiter.ts`

## File info
- Source path: `src/blob/persistence/PageWithDelimiter.ts`
- Source lines: `193`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/page_with_delimiter.rs`
- Crate: `azurite-blob`
- Module: `persistence::page_with_delimiter`
- Phase: `10.10`
- Status: `not_started`

## Exported API
### Default class `PageWithDelimiter<BlobType>`
- Constructor: `new PageWithDelimiter(maxResults: number, delimiter?: string, prefix?: string)`
- Public methods: `reset()`, `fill(reader, namer)`
- Internal helpers: `updateFull()`, `addItem()`, `addPrefix()`, `add()`, `processList()`, `prefixes()`
- Public state exposed for tests/callers: `blobItems`, `blobPrefixes`, `latestMarker`

## Dependencies
- `BlobPrefixModel` from `./IBlobMetadataStore`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| generic page buffer with delimiter logic | generic Rust struct over `T` | This is the hierarchical-listing companion to `FilterBlobPage`. |
| `Set<string>` of prefixes | ordered set / insertion-preserving collection | TS later converts the set into `BlobPrefixModel[]` in insertion order. |
| reader callback | async closure/fn taking `offset` | Same repeated-batch pagination pattern as `FilterBlobPage`. |

## Special handling
- `PageWithDelimiter.ts:29-37` stores both the delimiter and an optional prefix length so it can detect the next delimiter after the requested prefix.
- `PageWithDelimiter.ts:78-92` allows duplicate prefixes to keep being “added” even after the page is full, but a new prefix after fullness flips `isExhausted`.
- `PageWithDelimiter.ts:107-134` classifies each sorted name as either a blob item or a squashed prefix based on the first delimiter occurrence after `prefixLength`.
- `PageWithDelimiter.ts:162-181` returns a three-tuple `(blobItems, prefixes(), continuationToken)`, unlike `FilterBlobPage`'s two-tuple.
- `PageWithDelimiter.ts:184-191` materializes prefixes by iterating the internal `Set`, so result ordering follows insertion order rather than a separate sort pass.

## Change propagation notes
- If TS changes how delimiter squashing interacts with continuation tokens, update this helper and `LokiBlobMetadataStore.listBlobs()` together.
- Do not silently replace the duplicate-prefix-after-full behavior with a simpler cutoff; that would change observable paging results.
