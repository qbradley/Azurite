# Porting Record — `src/common/persistence/IExtentMetadataStore.ts`

## File info
- Source path: `src/common/persistence/IExtentMetadataStore.ts`
- Source lines: `108`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/persistence/i_extent_metadata_store.rs`
- Crate: `azurite-common`
- Module: `persistence::i_extent_metadata_store`
- Phase: `1.14`
- Status: `analyzed`

## Exported API
### Interface `IExtentModel`
- `id: string`
- `locationId: string`
- `path: string`
- `size: number`
- `lastModifiedInMS: number`

### Default interface `IExtentMetadataStore extends IGCExtentProvider, IDataStore, ICleaner`
- `updateExtent(extent: IExtentModel): Promise<void>`
- `deleteExtent(extentId: string): Promise<void>`
- `listExtents(id?: string, maxResults?: number, marker?: number | undefined, queryTime?: Date, protectTimeInMs?: number): Promise<[IExtentModel[], number | undefined]>`
- `getExtentLocationId(extentId: string): Promise<string>`
- Inherited from `IGCExtentProvider`: `iteratorExtents(): AsyncIterator<string[]>`

## Dependencies
- `../ICleaner`
- `../IDataStore`
- `../IGCExtentProvider`
- Implementations: `LokiExtentMetadataStore`, `SqlExtentMetadataStore`.
- Returned by: `IExtentStore.getMetadataStore()`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `string` | `String` | Metadata row values. |
| `number` size | `u64` | Stored byte size. |
| `number` timestamp | `i64` or `u64` | Millisecond epoch value. |
| `Date` | `chrono::DateTime<Utc>` or `std::time::SystemTime` | Query cutoff input. |
| `Promise<[IExtentModel[], number | undefined]>` | `async fn -> Result<(Vec<ExtentMetadataModel>, Option<u64>), StorageError>` | Keep tuple pagination. |
| `AsyncIterator<string[]>` | `BoxStream<'_, Vec<String>>` | Via `IGCExtentProvider`. |

## Recommended Rust translation
```rust
pub struct ExtentMetadataModel {
    pub id: String,
    pub location_id: String,
    pub path: String,
    pub size: u64,
    pub last_modified_in_ms: i64,
}

#[async_trait]
pub trait ExtentMetadataStore: GcExtentProvider + DataStore + Cleaner + Send + Sync {
    async fn update_extent(&mut self, extent: ExtentMetadataModel) -> Result<(), StorageError>;
    async fn delete_extent(&mut self, extent_id: &str) -> Result<(), StorageError>;
    async fn list_extents(
        &self,
        id: Option<&str>,
        max_results: Option<u64>,
        marker: Option<u64>,
        query_time: Option<chrono::DateTime<chrono::Utc>>,
        protect_time_in_ms: Option<u64>,
    ) -> Result<(Vec<ExtentMetadataModel>, Option<u64>), StorageError>;
    async fn extent_location_id(&self, extent_id: &str) -> Result<String, StorageError>;
}
```

## Special handling
- Keep this model distinct from the legacy `IExtentMetadata.IExtentModel`. The field names are meaningfully different: `locationId` vs `persistencyId`, and `lastModifiedInMS` vs `LastModifyInMS`.
- `LokiExtentMetadataStore` bridges the newer public interface to legacy Loki document fields by assigning `doc.LastModifyInMS = extent.lastModifiedInMS`. The Rust port should preserve that impedance mismatch explicitly.
- `iteratorExtents()` arrives from `IGCExtentProvider`; do not duplicate it into the translated trait unless a future TS change does.
- The `marker` value behaves like a pagination cursor and should stay optional in the Rust API.

## Change propagation notes
- Any field rename must be audited across both SQL and Loki implementations and compared against the legacy `IExtentMetadata` record.
- If TS changes the pagination tuple or iterator batching strategy, update GC manager expectations at the same time.
