# Porting Record — `src/common/persistence/LokiExtentMetadataStore.ts`

## File info
- Source path: `src/common/persistence/LokiExtentMetadataStore.ts`
- Source lines: `217`
- Source type: `handwritten`
- Rust target: `azurite-common/src/persistence/loki_extent_metadata_store.rs`
- Crate: `azurite-common`
- Module: `persistence::loki_extent_metadata_store`
- Phase: `2.4`
- Status: `ported`

## Exported API
### Class `LokiExtentMetadata implements IExtentMetadataStore`
- **Constructor**: `new LokiExtentMetadata(lokiDBPath: string, inMemory: boolean)`
- **Methods** (IExtentMetadataStore + IDataStore + ICleaner):
  - `isInitialized(): boolean`
  - `isClosed(): boolean`
  - `async init(): Promise<void>` (loads/creates DB, ensures collection exists)
  - `async close(): Promise<void>`
  - `async clean(): Promise<void>` (rimraf DB file)
  - `async updateExtent(extent: IExtentModel): Promise<void>` (upsert by id)
  - `async deleteExtent(extentId: string): Promise<void>`
  - `async listExtents(id?: string, maxResults?: number, marker?: number, queryTime?: Date, protectTimeInMs?: number): Promise<[IExtentModel[], number | undefined]>` (paginated query)
  - `async getExtentLocationId(extentId: string): Promise<string>` (fetch locationId field)
  - `iteratorExtents(): AsyncIterator<string[]>` (returns AllExtentsAsyncIterator)
- **Properties**:
  - `public readonly lokiDBPath: string`
  - `private readonly db: Loki`
  - `private initialized: boolean`
  - `private closed: boolean`
  - `private readonly EXTENTS_COLLECTION: string` = `"$EXTENTS_COLLECTION$"`

## Dependencies
- Imports: `fs.stat`, `lokijs`, `rimrafAsync`, `AllExtentsAsyncIterator`, `IExtentMetadataStore`, `IExtentModel`
- Porting status:
  - `Loki` (lokijs): Embedded DB; Rust equivalent: `sled`, `rocksdb`, or in-memory Map
  - `fs.stat`: `tokio::fs::metadata()`
  - `rimrafAsync`: external crate
  - `AllExtentsAsyncIterator`: Phase 2.5 (analyzed)
  - `IExtentMetadataStore`, `IExtentModel`: Phase 1.14 (analyzed)
- Important consumers: FSExtentStore, MemoryExtentStore (metadata queries during append/delete/read)

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `Loki` instance | `sled::Db` or `Arc<Mutex<HashMap<String, IExtentModel>>>` | Loki is embedded; use sled or in-memory. |
| Loki Collection (typed) | `sled::Tree` or `HashMap<String, IExtentModel>` | Keyed storage. |
| `.find().limit().data()` chain | Iterator + collect, or custom query builder | Adapt to Rust. |
| `$loki` (internal ID) | Custom `marker` field or vec index | Pagination support. |
| `Date` (queryTime) | `chrono::DateTime<Utc>` or `SystemTime` | Timestamp for GC. |
| `Promise<void>` (callback-based) | `async fn -> Result<(), StorageError>` | Async/await. |
| `AsyncIterator<string[]>` | `Box<dyn Stream<...> + Send>` or impl | Delegate to AllExtentsAsyncIterator. |

## Recommended Rust translation
```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::json;

pub struct LokiExtentMetadata {
    pub loki_db_path: String,
    db: Arc<Mutex<HashMap<String, IExtentModel>>>,
    initialized: Arc<AtomicBool>,
    closed: Arc<AtomicBool>,
    in_memory: bool,
}

impl LokiExtentMetadata {
    pub fn new(loki_db_path: String, in_memory: bool) -> Self { /* ... */ }

    pub async fn init(&mut self) -> Result<(), StorageError> {
        // Check DB file existence.
        // Load or initialize storage.
        // Ensure collection/table exists.
        // ...
    }

    pub async fn update_extent(&mut self, extent: &IExtentModel) -> Result<(), StorageError> {
        // Upsert: find by id, update size/lastMod, or insert new.
        // ...
    }

    pub async fn list_extents(
        &self,
        id: Option<&str>,
        max_results: Option<usize>,
        marker: Option<u64>,
        query_time: Option<DateTime<Utc>>,
        protect_time_in_ms: Option<u64>,
    ) -> Result<(Vec<IExtentModel>, Option<u64>), StorageError> {
        // Build filters (id, time, marker).
        // Apply limit.
        // Return docs + next_marker.
        // ...
    }

    pub async fn iterator_extents(&self) -> impl Stream<Item = Vec<String>> {
        AllExtentsAsyncIterator::new(Arc::new(self.clone()))
    }
}

#[async_trait]
impl IExtentMetadataStore for LokiExtentMetadata { /* ... */ }
```

## Special handling
- **`LastModifyInMS` / `lastModifiedInMS` naming mismatch (FIDELITY RISK)**: In `updateExtent()`, the TS source writes `doc.LastModifyInMS = extent.lastModifiedInMS`. The stored Loki field is `LastModifyInMS` (PascalCase, no `d`) but the `IExtentModel` interface exposes `lastModifiedInMS` (camelCase, with `d`). The `listExtents()` query also filters on `LastModifyInMS`. The Rust port must preserve this exact field name distinction: the stored/queried field is `LastModifyInMS`; the interface field is `lastModifiedInMS`. Do not unify them without an explicit compatibility layer.
- **Loki abstraction**: TS uses Loki embedded DB. Rust options:
  1. **sled**: Embedded KV store, ACID, closest match (recommended).
  2. **rocksdb**: Higher performance, heavier dependency.
  3. **In-memory HashMap + optional JSON serialization**: Simple for Phase 2.
  - Choose sled for Phase 2.3+ for persistence without complexity.
- **Collection/document pattern**: Loki uses typed collections. Rust should map to sled Trees or HashMap with key prefixes.
- **$loki pagination field**: Loki auto-increments this. Rust should either:
  1. Add explicit `marker` field to IExtentModel, or
  2. Maintain insertion order (vec index) and track as marker.
- **Callback-based API**: Loki accepts callbacks (`db.loadDatabase({}, (err) => { ... })`). Rust uses async/await; wrap fs I/O with tokio.
- **AllExtentsAsyncIterator delegation**: Returns iterator instance that repeatedly calls `listExtents()` with pagination. Rust must ensure Arc sharing.

## Control flow notes
1. **init()**:
   - If in_memory: skip file checks, create empty HashMap.
   - If on-disk: stat lokiDBPath. If exists, load DB. If not, skip (create on first insert).
   - Ensure collection/tree exists (create with indices if sled).
   - Persist metadata (sled auto-saves; HashMap needs explicit save if desired).

2. **updateExtent(extent)**:
   - Find document by id in collection.
   - If not found: insert entire document.
   - If found: update `size` and **`LastModifyInMS`** (not `lastModifiedInMS` — use stored field name) from `extent.lastModifiedInMS`, persist.

3. **listExtents(id, maxResults, marker, queryTime, protectTimeInMs)**:
   - Build query filters: `id`, **`LastModifyInMS`** `< (queryTime.millis - protectTimeInMs)`, marker (using Loki `$loki` field).
   - Execute query with limit.
   - Return (docs, next_marker): if docs.len < maxResults, no next marker; else extract from last doc.

4. **deleteExtent(extentId)**:
   - Find and remove by id.

5. **iteratorExtents()**:
   - Return `AllExtentsAsyncIterator::new(self)`.

## Change propagation notes
- If Loki is replaced with different DB, rework init/query/persist.
- If IExtentModel schema changes, update updateExtent() to cover all fields.
- If pagination semantics shift (time-based instead of $loki), update listExtents() and AllExtentsAsyncIterator.
- If in-memory mode is deprecated, remove that branch.

## Rust port notes
- Ported to `azurite-common/src/persistence/loki_extent_metadata_store.rs` as a JSON-backed custom document store (`Vec` + `RwLock`) instead of a LokiJS port, per STRATEGY §11.1.
- Preserved the `LastModifyInMS` storage field separately from `IExtentModel.lastModifiedInMS`, and retained marker-based pagination for `AllExtentsAsyncIterator`.
- The port now implements `IGCExtentProvider` directly and was validated with `cargo check` and `cargo test -p azurite-common`.

