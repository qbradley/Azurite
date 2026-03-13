# Porting Record — `src/common/persistence/FSExtentStore.ts`

## File info
- Source path: `src/common/persistence/FSExtentStore.ts`
- Source lines: `677`
- Source type: `handwritten`
- Rust target: `azurite-common/src/persistence/fs_extent_store.rs`
- Crate: `azurite-common`
- Module: `persistence::fs_extent_store`
- Phase: `2.3`
- Status: `ported`

## Exported API
### Class `FSExtentStore implements IExtentStore`
- **Constructor**: `new FSExtentStore(metadata: IExtentMetadataStore, persistencyConfiguration: StoreDestinationArray, logger: ILogger)`
- **Public methods** (IExtentStore):
  - `isInitialized(): boolean`
  - `isClosed(): boolean`
  - `async init(): Promise<void>` (creates paths, inits metadata)
  - `async close(): Promise<void>`
  - `async clean(): Promise<void>` (rimraf all paths)
  - `async appendExtent(data: NodeJS.ReadableStream | Buffer, contextId?: string): Promise<IExtentChunk>`
  - `async readExtent(extentChunk?: IExtentChunk, contextId?: string): Promise<NodeJS.ReadableStream>`
  - `async readExtents(extentChunkArray: IExtentChunk[], offset: number, count: number, contextId?: string): Promise<NodeJS.ReadableStream>`
  - `async deleteExtents(extents: Iterable<string>): Promise<number>`
  - `getMetadataStore(): IExtentMetadataStore`
- **Private methods**:
  - `async streamPipe(rs: NodeJS.ReadableStream, ws: Writable, fd?: number, contextId?: string): Promise<number>` (pipes + syncs)
  - `isActiveExtent(id: string): boolean` (linear search)
  - `createAppendExtent(persistencyId: string): IAppendExtent`
  - `getNewExtent(appendExtent: IAppendExtent): void` (in-place reset)
  - `generateExtentPath(persistencyId: string, extentId: string): string`
- **Properties**:
  - `private readonly metadataStore: IExtentMetadataStore`
  - `private readonly appendQueue: IOperationQueue` (max concurrency = total activeWriteExtents)
  - `private readonly readQueue: IOperationQueue` (max concurrency = DEFAULT_READ_CONCURRENCY)
  - `private initialized: boolean`
  - `private closed: boolean`
  - `private activeWriteExtents: IAppendExtent[]` (pool, one per location+concurrency slot)
  - `private activeWriteExtentsNumber: number`
  - `private persistencyPath: Map<string, string>` (locationId -> path)

### Private enum `AppendStatusCode`
- `Idle = 0`
- `Appending = 1`

### Private interface `IAppendExtent`
- `id: string`
- `offset: number`
- `appendStatus: AppendStatusCode`
- `locationId: string`
- `fd?: number` (cached file descriptor)

## Dependencies
- Imports: `fs.{close, open, stat, unlink, mkdir, truncate, fdatasync, createReadStream, createWriteStream}`, `path.join`, `stream.Writable`, `util.promisify`, `uuid`, `multistream`, `Buffer`, `ILogger`, `ZeroBytesStream`, `BufferStream`, `rimrafAsync`, `IExtentMetadataStore`, `IExtentStore`, `IOperationQueue`, `OperationQueue`, constants, `ZERO_EXTENT_ID` (from blob layer)
- Porting status:
  - `fs` ops: `tokio::fs` equivalents
  - `multistream`: futures combinator
  - `uuid`: external
  - `ILogger`, `IExtentMetadataStore`, `IExtentStore`, `IOperationQueue`, `OperationQueue`: Phase 1/2 analyzed
  - `ZeroBytesStream`: Phase 2.6 (analyzed)
  - `BufferStream`: internal helper
  - **`ZERO_EXTENT_ID` (FIDELITY RISK — cross-crate boundary)**: Imported from `../../blob/persistence/IBlobMetadataStore`. In TS monorepo this is fine; in Rust, `azurite-common` must not depend on `azurite-blob`. This constant (`"*ZERO*"`) must be moved to `azurite-common` (or replicated) to break the cycle. Flag for Aragorn.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `IAppendExtent` with mutable fd | `Arc<Mutex<AppendExtent>>` per location | Shared state across tasks. |
| `fd?: number` | `Option<std::fs::File>` or RawFd | RAII lifecycle. |
| `NodeJS.ReadableStream \| Buffer` | `enum ExtentDataInput { ... }` | Preserve union. |
| `Writable` stream | `tokio::io::AsyncWrite` | Async I/O. |
| `promisify(fs.*)` | Direct `tokio::fs::*` async calls | Native async/await. |
| `Map<string, string>` | `HashMap<String, PathBuf>` | Immutable after init. |
| `IAppendExtent[]` pool | `Vec<Arc<Mutex<AppendExtent>>>` | Shared mutable. |
| `fdatasync` callback | `tokio::fs::File::sync_data()` or `nix::fcntl::fsync()` | Explicit sync call. |

## Recommended Rust translation
```rust
#[derive(Clone)]
enum AppendStatusCode {
    Idle,
    Appending,
}

pub struct AppendExtent {
    pub id: String,
    pub offset: u64,
    pub append_status: AppendStatusCode,
    pub location_id: String,
    pub file: Option<tokio::fs::File>,
}

pub struct FSExtentStore {
    metadata_store: Arc<dyn IExtentMetadataStore + Send + Sync>,
    append_queue: Arc<OperationQueue>,
    read_queue: Arc<OperationQueue>,
    
    initialized: Arc<AtomicBool>,
    closed: Arc<AtomicBool>,
    
    active_write_extents: Arc<Mutex<Vec<Arc<Mutex<AppendExtent>>>>>,
    active_write_extents_number: usize,
    persistency_path: HashMap<String, PathBuf>,
}

#[async_trait]
impl IExtentStore for FSExtentStore {
    async fn append_extent(
        &mut self,
        data: ExtentDataInput,
        context_id: Option<&str>,
    ) -> Result<ExtentChunk, StorageError> { /* ... */ }

    async fn stream_pipe(
        &self,
        mut rs: impl AsyncRead + Unpin,
        mut ws: impl AsyncWrite + Unpin,
        file: Option<&tokio::fs::File>,
        context_id: Option<&str>,
    ) -> Result<u64, StorageError> {
        let mut total = 0u64;
        let mut buf = vec![0u8; 8192];
        loop {
            match rs.read(&mut buf).await? {
                0 => break,
                n => {
                    ws.write_all(&buf[..n]).await?;
                    total += n as u64;
                }
            }
        }
        ws.flush().await?;
        if let Some(f) = file {
            f.sync_data().await?;
        }
        Ok(total)
    }
}
```

## Special handling
- **activeWriteExtents pool**: Pre-allocated pool (one per location × maxConcurrency). Each wrapped in `Arc<Mutex<AppendExtent>>` for shared mutable access.
- **File descriptor caching**: TS caches `fd` on extent. Rust must manage File handle lifetime carefully; consider dropping on error and reopening.
- **OperationQueue concurrency**: appendQueue limited by pool size; readQueue by DEFAULT_READ_CONCURRENCY. Both serialize operations via queues.
- **streamPipe() complexity**: Handles bidirectional stream piping with explicit sync. Rust equivalent: manual copy loop + explicit `sync_data()` call.
- **process.nextTick() semantics**: TS defers queue continuation. Rust should spawn background task or rely on queue to naturally feed next op.
- **isActiveExtent() performance**: O(N) linear search. Could use HashSet if many extents.

## Control flow notes
1. **Constructor**: For each StoreDestinationArray config, create maxConcurrency AppendExtent instances. Init appendQueue (limited by total extents) and readQueue (DEFAULT_READ_CONCURRENCY).

2. **appendExtent(data, contextId)**:
   - Wrap in closure and enqueue on appendQueue.
   - Scan activeWriteExtents for Idle extent (first found).
   - If offset >= MAX_EXTENT_SIZE, close fd and call getNewExtent() (reset).
   - Convert data to stream, open fd in append mode (or reuse cached).
   - Call streamPipe() to write; get byte count.
   - Create IExtentModel, update metadata.
   - On success: mark Idle, return ExtentChunk. On error: truncate, mark Idle, throw.

3. **readExtent(extentChunk, contextId)**:
   - If undefined or count=0, return ZeroBytesStream(0).
   - Get locationId from metadata, generate path.
   - Create read stream, wrap in closure, enqueue on readQueue.

4. **readExtents(chunks, offset, count, contextId)**:
   - Compute logical byte ranges.
   - For each chunk, call readExtent() with adjusted offsets.
   - Combine streams.

5. **deleteExtents(extents)**:
   - Skip if isActiveExtent(id). Otherwise, unlink file and delete metadata.

6. **streamPipe(rs, ws, file, contextId)**:
   - Manual copy loop: read into buffer, write to ws, count bytes.
   - On finish: call file.sync_data() if present.
   - Propagate errors from both streams.

## Change propagation notes
- If maxConcurrency per-location changes, rework extent pool initialization.
- If StoreDestinationArray structure changes, update path generation.
- If activeWriteExtents becomes dynamically allocated, refactor selection logic.
- If streamPipe() must support cancellation, add cancellation check in loop.
- If OperationQueue concurrency semantics change, revisit queue limits.
- If fd caching auto-closes (timeout), update fd lifecycle management.

## Rust port notes
- Ported to `azurite-common/src/persistence/fs_extent_store.rs` with Tokio file I/O, a per-location append-extent pool, cached `tokio::fs::File` handles, and two `OperationQueue` instances matching the TS append/read separation.
- Preserved the `ZERO_EXTENT_ID` handling and the truncate-on-write-error recovery path; the active extent pool still rotates to a fresh UUID when an extent reaches `DEFAULT_MAX_EXTENT_SIZE`.
- Multi-extent reads still flow through repeated `readExtent()` calls and a concatenated async reader; validated with `cargo check` and `cargo test -p azurite-common`.

