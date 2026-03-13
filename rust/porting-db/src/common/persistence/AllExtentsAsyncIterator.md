# Porting Record — `src/common/persistence/AllExtentsAsyncIterator.ts`

## File info
- Source path: `src/common/persistence/AllExtentsAsyncIterator.ts`
- Source lines: `45`
- Source type: `handwritten`
- Rust target: `azurite-common/src/persistence/all_extents_async_iterator.rs`
- Crate: `azurite-common`
- Module: `persistence::all_extents_async_iterator`
- Phase: `2.5`
- Status: `ported`

## Exported API
### Class `AllExtentsAsyncIterator implements AsyncIterator<string[]>`
- **Constructor**: `new AllExtentsAsyncIterator(extentMetadata: IExtentMetadataStore)`
- **Method** (AsyncIterator protocol):
  - `async next(): Promise<IteratorResult<string[]>>`
    - Returns `{ done: false, value: string[] }` on next batch.
    - Returns `{ done: true, value: [] }` on exhaustion.
- **Properties**:
  - `private unit: number` = 1000 (batch size)
  - `private done: boolean` (exhausted flag)
  - `private marker?: number` (pagination cursor)
  - `private readonly time: Date` (snapshot time for GC protect-time filter)
  - `private readonly extentMetadata: IExtentMetadataStore` (delegated store)

## Dependencies
- Imports: `./IExtentMetadataStore`, `../utils/constants.DEFAULT_EXTENT_GC_PROTECT_TIME_IN_MS`
- Porting status:
  - `IExtentMetadataStore`: Phase 1.14 (analyzed)
  - `DEFAULT_EXTENT_GC_PROTECT_TIME_IN_MS`: constant (~1 hour in ms)
- Important consumers: LokiExtentMetadataStore.iteratorExtents(), blob/queue GC managers

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `AsyncIterator<string[]>` | `impl Stream<Item = Vec<String>>` or custom struct | Use futures::stream or custom. |
| `IteratorResult<string[]>` | `Option<Vec<String>>` or `struct { value, done }` | Rust Stream returns Option; custom impl needs explicit done. |
| `Date` | `chrono::DateTime<Utc>` or `std::time::SystemTime` | Immutable snapshot. |
| `marker?: number` | `Option<u64>` | Pagination cursor. |
| `unit: number` | `const UNIT: usize = 1000` | Immutable batch size. |

## Recommended Rust translation
```rust
use chrono::{DateTime, Utc};
use futures::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct AllExtentsAsyncIterator {
    unit: usize,
    done: bool,
    marker: Option<u64>,
    time: DateTime<Utc>,
    extent_metadata: Arc<dyn IExtentMetadataStore + Send + Sync>,
}

impl AllExtentsAsyncIterator {
    pub fn new(extent_metadata: Arc<dyn IExtentMetadataStore + Send + Sync>) -> Self {
        AllExtentsAsyncIterator {
            unit: 1000,
            done: false,
            marker: None,
            time: Utc::now(),
            extent_metadata,
        }
    }

    pub async fn next_batch(&mut self) -> Option<Vec<String>> {
        if self.done {
            return None;
        }

        match self.extent_metadata.list_extents(
            None,
            Some(self.unit),
            self.marker,
            Some(self.time),
            Some(DEFAULT_EXTENT_GC_PROTECT_TIME_IN_MS),
        ).await {
            Ok((extents, next_marker)) => {
                self.marker = next_marker;
                if next_marker.is_none() {
                    self.done = true;
                }
                Some(extents.into_iter().map(|e| e.id).collect())
            }
            Err(_) => {
                self.done = true;
                None
            }
        }
    }
}

// Alternative: implement Stream trait
impl Stream for AllExtentsAsyncIterator {
    type Item = Result<Vec<String>, StorageError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Use tokio::task::block_in_place or spawn_blocking for async call
        // Or refactor to use a future-based state machine
        // ...
    }
}
```

## Special handling
- **AsyncIterator protocol**: TS AsyncIterator with `next(): Promise<IteratorResult<T>>`. Rust options:
  1. `futures::stream::Stream` trait (preferred): `poll_next()` with Pin/Context.
  2. Custom struct with `async fn next_batch()` method.
  3. `#[async_trait]` macro for easier syntax.
  - Phase 2 should use Stream trait for compatibility with Tokio ecosystem.
- **Snapshot time**: Iterator captures `new Date()` at construction. All queries use this snapshot to ensure consistent GC protection. Rust must preserve this immutability.
- **Pagination protocol**: Iterator maintains `marker` state. Each `next()` call fetches one batch (unit=1000 extents). When `nextMarker` is undefined, iteration is exhausted and `done=true`.
- **Reference sharing**: Iterator holds Arc to IExtentMetadataStore; must support concurrent access across async boundaries.

## Control flow notes
1. **Constructor**: Capture current time, initialize done=false, marker=None, unit=1000.
2. **next()**:
   - If done, return None (or `{ done: true, value: [] }`).
   - Call `extent_metadata.list_extents(None, Some(1000), marker, time, GC_PROTECT_MS)`.
   - Update marker from result.
   - If nextMarker is None, set done=true.
   - Return Some with batch of extent IDs.

## Change propagation notes
- If batch size (unit) becomes configurable per-instance, add constructor parameter.
- If GC protect-time becomes per-container or per-call, pass as parameter.
- If listExtents() pagination semantics change, update next() logic.
- If IExtentMetadataStore trait signature changes, update method calls.

## Rust port notes
- Ported to `azurite-common/src/persistence/all_extents_async_iterator.rs` as a stateful iterator struct plus `into_stream()` adapter for the existing `BoxStream` trait boundary.
- Preserved snapshot-time behavior (`Utc::now()` captured once at construction) and the 1000-item batch size.
- Validated together with the metadata-store port using `cargo check` and `cargo test -p azurite-common`.

