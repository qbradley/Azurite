# Porting Record — `src/common/persistence/MemoryExtentStore.ts`

## File info
- Source path: `src/common/persistence/MemoryExtentStore.ts`
- Source lines: `394`
- Source type: `handwritten`
- Rust target: `azurite-common/src/persistence/memory_extent_store.rs`
- Crate: `azurite-common`
- Module: `persistence::memory_extent_store`
- Phase: `2.2`
- Status: `analyzed`

## Exported API
### Interface `IMemoryExtentChunk extends IExtentChunk`
- `chunks: (Buffer | string)[]`

### Private interface `IExtentCategoryChunks`
- `chunks: Map<string, IMemoryExtentChunk>` (extent ID → chunk)
- `totalSize: number` (sum of all chunk `count` values in this category)

### Class `MemoryExtentChunkStore`
- **Constructor**: `new MemoryExtentChunkStore(sizeLimit?: number)`
- **Methods**:
  - `clear(categoryName: string): void`
  - `set(categoryName: string, chunk: IMemoryExtentChunk): void` (throws on limit)
  - `trySet(categoryName: string, chunk: IMemoryExtentChunk): boolean`
  - `get(categoryName: string, id: string): IMemoryExtentChunk | undefined`
  - `delete(categoryName: string, id: string): boolean`
  - `totalSize(): number`
  - `setSizeLimit(sizeLimit?: number): boolean`
  - `sizeLimit(): number | undefined`
- **Properties**:
  - `private _sizeLimit?: number`
  - `private readonly _chunks: Map<string, IExtentCategoryChunks>`
  - `private _totalSize: number`

### Exported constant `DEFAULT_EXTENT_MEMORY_LIMIT`
- `DEFAULT_EXTENT_MEMORY_LIMIT: number = Math.trunc(totalmem() * 0.5)` (50% of RAM)

### Exported constant `SharedChunkStore`
- `SharedChunkStore: MemoryExtentChunkStore = new MemoryExtentChunkStore(DEFAULT_EXTENT_MEMORY_LIMIT)`

### Default class `MemoryExtentStore implements IExtentStore`
- **Constructor**: `new MemoryExtentStore(categoryName: string, chunks: MemoryExtentChunkStore, metadata: IExtentMetadataStore, logger: ILogger, makeError: (statusCode: number, storageErrorCode: string, storageErrorMessage: string, storageRequestID: string) => Error)`
- **Methods**:
  - `isInitialized(): boolean`
  - `isClosed(): boolean`
  - `init(): Promise<void>`
  - `close(): Promise<void>`
  - `clean(): Promise<void>`
  - `appendExtent(data: NodeJS.ReadableStream | Buffer, contextId?: string): Promise<IExtentChunk>`
  - `readExtent(extentChunk?: IExtentChunk, contextId?: string): Promise<NodeJS.ReadableStream>`
  - `readExtents(extentChunkArray: IExtentChunk[], offset: number, count: number, contextId?: string): Promise<NodeJS.ReadableStream>`
  - `deleteExtents(extents: Iterable<string>): Promise<number>`
  - `getMetadataStore(): IExtentMetadataStore`
- **Properties**:
  - `private readonly categoryName: string`
  - `private readonly chunks: MemoryExtentChunkStore`
  - `private readonly metadataStore: IExtentMetadataStore`
  - `private readonly logger: ILogger`
  - `private readonly makeError: (statusCode: number, storageErrorCode: string, storageErrorMessage: string, storageRequestID: string) => Error`
  - `private initialized: boolean`
  - `private closed: boolean`

## Dependencies
- Imports: `../../blob/persistence/IBlobMetadataStore.ZERO_EXTENT_ID`, `../ILogger`, `../ZeroBytesStream`, `./IExtentMetadataStore`, `./IExtentStore`, `uuid`, `multistream`, `stream.Readable`, `os.totalmem`
- Porting status:
  - `IExtentStore`: analyzed (`1.13`)
  - `IExtentMetadataStore`: analyzed (`1.14`)
  - `ZeroBytesStream`: analyzed (`2.6`)
  - `ILogger`: analyzed (`1.3`)
  - `uuid`: external dependency
  - `multistream`: replace with a sequential stream combiner in Rust
  - `stream.Readable`: replace with `AsyncRead`-compatible reader type
  - `os.totalmem`: use `sysinfo` or equivalent host-memory probe
  - **`ZERO_EXTENT_ID` (FIDELITY RISK — cross-crate boundary)**: Imported from `../../blob/persistence/IBlobMetadataStore`. `azurite-common` must not depend on `azurite-blob`. Move or replicate constant (`"*ZERO*"`) in `azurite-common` while preserving the exact value.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `Map<string, IExtentCategoryChunks>` | `HashMap<String, ExtentCategoryChunks>` | Preserve the two-level category wrapper; it tracks both per-id chunks and category-local totals. |
| `Map<string, IMemoryExtentChunk>` | `HashMap<String, MemoryExtentChunk>` | Inner map is keyed by extent id. |
| `(Buffer \| string)[]` | `Vec<ExtentSegment>` or `Vec<bytes::Bytes>` with binary-only assumption documented | Current callers are binary, but the TS type admits string chunks. |
| `NodeJS.ReadableStream \| Buffer` | `enum ExtentDataInput { Buffer(bytes::Bytes), Stream(Pin<Box<dyn AsyncRead + Send>>) }` | Preserve the union input boundary. |
| `Readable` accumulator stream | custom `AsyncRead` over queued chunk slices | `readExtent()` pushes slices into a synthetic stream after applying skip/take math. |
| `multistream` | sequential reader/stream combiner | Preserve concatenation order across extent chunks. |
| `Iterable<string>` | `Vec<String>` / `&[String]` / generic iterator at API edge | Consumer contract is iteration, not random access. |

## Recommended Rust translation
```rust
pub struct MemoryExtentChunk {
    pub id: String,
    pub offset: u64,
    pub count: u64,
    pub chunks: Vec<bytes::Bytes>,
}

pub struct MemoryExtentChunkStore {
    size_limit: Option<u64>,
    chunks: Arc<RwLock<HashMap<String, HashMap<String, MemoryExtentChunk>>>>,
    total_size: Arc<AtomicU64>,
}

pub const DEFAULT_EXTENT_MEMORY_LIMIT: u64 = /* 50% of sys memory */;

lazy_static::lazy_static! {
    pub static ref SHARED_CHUNK_STORE: MemoryExtentChunkStore = 
        MemoryExtentChunkStore::new(Some(DEFAULT_EXTENT_MEMORY_LIMIT));
}

#[async_trait]
impl IExtentStore for MemoryExtentStore {
    async fn append_extent(...) -> Result<ExtentChunk, StorageError> { /* ... */ }
    async fn read_extent(...) -> Result<impl AsyncRead + Send, StorageError> { /* ... */ }
    async fn read_extents(...) -> Result<impl AsyncRead + Send, StorageError> { /* ... */ }
    async fn delete_extents(...) -> Result<u64, StorageError> { /* ... */ }
}
```

## Special handling
- **SharedChunkStore singleton**: Use `lazy_static!` or `once_cell` so all in-memory stores share one global extent map, just like the TS module-level export.
- **Two-level accounting**: `IExtentCategoryChunks` stores both the per-id map and a category-local `totalSize`; preserve that wrapper instead of flattening directly to a single map.
- **Memory limit atomicity**: `trySet()` computes a delta against an existing extent before enforcing the global size limit. Keep that replace-in-place accounting exact.
- **Synthetic read stream**: `readExtent()` builds a fresh `Readable`, applies `skip` / `take` over each stored chunk, and only then returns it. Preserve the slice-by-slice behavior.
- **`Math.min(extentChunk.count)` oddity**: the `ZERO_EXTENT_ID` branch passes a single argument to `Math.min`, which is effectively a no-op. Preserve behavior rather than silently “fixing” it.
- **Error construction callback**: constructor accepts a service-specific `makeError(...) => Error`; keep it as an injected callback boundary instead of baking storage errors into the common layer.

## Control flow notes
1. **appendExtent**: consume stream/buffer, accumulate chunks, store in map, update metadata, return extent.
2. **readExtent**: fetch stored chunks, iterate respecting offset/count, emit slices via stream.
3. **readExtents**: combine multiple readExtent() results.
4. **deleteExtents**: iterate IDs, delete from store and metadata.

## Change propagation notes
- If memory limit uses LRU eviction instead of hard failure, refactor `trySet()`.
- If metadata store contract changes, sync `appendExtent()` logic.
- If SharedChunkStore becomes per-instance, refactor initialization.

