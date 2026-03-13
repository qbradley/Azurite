# Porting Record — `src/common/IGCExtentProvider.ts`

## File info
- Source path: `src/common/IGCExtentProvider.ts`
- Source lines: `11`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/i_gc_extent_provider.rs`
- Crate: `azurite-common`
- Module: `i_gc_extent_provider`
- Phase: `1.9`
- Status: `ported`

## Exported API
### Default interface `IGCExtentProvider extends IDataStore`
- `iteratorExtents(): AsyncIterator<string[]>`

## Dependencies
- `./IDataStore`
- Important downstream users: blob and queue GC managers; `IExtentMetadataStore` extends this trait.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `AsyncIterator<string[]>` | `BoxStream<'_, Vec<String>>` | Preserve batched lazy enumeration. |
| `string[]` | `Vec<String>` | Batch of extent IDs. |

## Recommended Rust translation
```rust
use futures::stream::BoxStream;

#[async_trait]
pub trait GcExtentProvider: DataStore + Send + Sync {
    fn iterator_extents(&self) -> BoxStream<'_, Vec<String>>;
}
```

## Special handling
- Preserve batching: TS yields `string[]`, not one extent ID at a time.
- This method is used by mark-and-sweep style GC loops, so the Rust port should favor streaming/lazy iteration over eagerly collecting all IDs.
- Keep the exact method-name distinction in notes: this trait uses `iteratorExtents()`, while legacy `IExtentMetadata` uses `getExtentIterator()` for similar semantics.

## Change propagation notes
- If the batch item type changes, re-check both GC managers and `IExtentMetadataStore` implementations.
- If TS changes from iterator to callback/event style, revisit every GC scanning loop before altering the Rust API.

## Rust port notes
- Ported to `rust/crates/azurite-common/src/i_gc_extent_provider.rs` with `BoxStream` batches.
- Kept batched `Vec<String>` streaming rather than collapsing to one extent ID per item.
