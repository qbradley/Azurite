# Porting Record — `src/common/persistence/IExtentStore.ts`

## File info
- Source path: `src/common/persistence/IExtentStore.ts`
- Source lines: `95`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/persistence/i_extent_store.rs`
- Crate: `azurite-common`
- Module: `persistence::i_extent_store`
- Phase: `1.13`
- Status: `analyzed`

## Exported API
### Interface `IExtentChunk`
- `id: string`
- `offset: number`
- `count: number`

### Exported type alias `StoreDestinationArray`
- `IStoreDestinationConfigure[]`

### Default interface `IExtentStore extends IDataStore, ICleaner`
- `appendExtent(data: NodeJS.ReadableStream | Buffer, contextId?: string): Promise<IExtentChunk>`
- `readExtent(extentChunk?: IExtentChunk, contextId?: string): Promise<NodeJS.ReadableStream>`
- `readExtents(extentChunkArray: IExtentChunk[], offset: number, count: number, contextId?: string): Promise<NodeJS.ReadableStream>`
- `deleteExtents(persistency: Iterable<string>): Promise<number>`
- `getMetadataStore(): IExtentMetadataStore`

## Dependencies
- `../ICleaner`
- `../IDataStore`
- `./IExtentMetadataStore`
- Important implementations: `FSExtentStore`, `MemoryExtentStore`.
- Hot-path consumers: blob and queue handlers, blob and queue GC managers.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `number` byte counts | `u64` | Offsets and sizes are byte-oriented and non-negative. |
| `NodeJS.ReadableStream` | `Pin<Box<dyn AsyncRead + Send>>` | Matches strategy section 6.4. |
| `Buffer` | `bytes::Bytes` or `Vec<u8>` | Prefer `Bytes` when zero-copy sharing matters. |
| `NodeJS.ReadableStream | Buffer` | `enum ExtentDataInput` | Required to preserve the union shape cleanly. |
| `Iterable<string>` | `Vec<String>` or `&[String]` at trait boundary | Avoid generic trait methods if object safety matters. |
| `IExtentMetadataStore` | `Arc<dyn ExtentMetadataStore + Send + Sync>` | Returned store is shared live state. |

## Recommended Rust translation
```rust
pub struct ExtentChunk {
    pub id: String,
    pub offset: u64,
    pub count: u64,
}

pub struct StoreDestinationConfig {
    pub location_path: std::path::PathBuf,
    pub location_id: String,
    pub max_concurrency: usize,
}

pub type StoreDestinationArray = Vec<StoreDestinationConfig>;

pub enum ExtentDataInput {
    Buffer(bytes::Bytes),
    Stream(std::pin::Pin<Box<dyn tokio::io::AsyncRead + Send>>),
}

#[async_trait]
pub trait ExtentStore: DataStore + Cleaner + Send + Sync {
    async fn append_extent(
        &mut self,
        data: ExtentDataInput,
        context_id: Option<&str>,
    ) -> Result<ExtentChunk, StorageError>;
    async fn read_extent(
        &self,
        extent_chunk: Option<&ExtentChunk>,
        context_id: Option<&str>,
    ) -> Result<std::pin::Pin<Box<dyn tokio::io::AsyncRead + Send>>, StorageError>;
    async fn read_extents(
        &self,
        extent_chunks: &[ExtentChunk],
        offset: u64,
        count: u64,
        context_id: Option<&str>,
    ) -> Result<std::pin::Pin<Box<dyn tokio::io::AsyncRead + Send>>, StorageError>;
    async fn delete_extents(
        &mut self,
        extent_ids: Vec<String>,
    ) -> Result<u64, StorageError>;
    fn metadata_store(&self) -> std::sync::Arc<dyn ExtentMetadataStore + Send + Sync>;
}
```

## Special handling
- Keep `contextId` casing notes separate from logger `contextID`; the persistence subsystem consistently uses lowercase `d`.
- `appendExtent()` accepts either a full buffer or a streaming body. Preserve the union rather than forcing all callers to materialize data in memory.
- `readExtent(undefined, contextId)` is used in blob handlers to represent an empty stream path; the Rust port should preserve the optional extent argument.
- `StoreDestinationArray` is the only exported view of `IStoreDestinationConfigure`; keep the hidden helper struct relationship visible in docs even if the Rust type is made public for convenience.

## Change propagation notes
- Any change to extent chunk shape or stream semantics affects blob handlers, queue handlers, and both GC managers.
- If TS changes `deleteExtents()` to accept a different iterable contract, revisit object-safety choices in the Rust trait surface.
