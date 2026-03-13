# Porting Record — `src/common/persistence/IExtentMetadata.ts`

## File info
- Source path: `src/common/persistence/IExtentMetadata.ts`
- Source lines: `77`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/persistence/i_extent_metadata.rs`
- Crate: `azurite-common`
- Module: `persistence::i_extent_metadata`
- Phase: `1.12`
- Status: `analyzed`

## Exported API
### Interface `IExtentModel`
- `id: string`
- `persistencyId: string`
- `path: string`
- `size: number`
- `LastModifyInMS: number`

### Default interface `IExtentMetadata extends IDataStore`
- `updateExtent(extent: IExtentModel): Promise<void>`
- `listExtents(id?: string, maxResults?: number, marker?: number, queryTime?: Date, UnmodifiedTime?: number): Promise<[IExtentModel[], number | undefined]>`
- `getExtentIterator(): AsyncIterator<string[]>`
- `deleteExtent(extentId: string): Promise<void>`
- `getExtentPersistencyId(extentId: string): Promise<string>`

## Dependencies
- `../IDataStore`
- No active implementation was found in the current tree; the codebase appears to use `IExtentMetadataStore` instead.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `string` | `String` | IDs and paths are owned values in metadata rows. |
| `number` size | `u64` | Byte sizes and pagination markers are non-negative. |
| `number` timestamp | `i64` or `u64` | Millisecond epoch value; document exact signedness choice. |
| `Date` | `chrono::DateTime<Utc>` or `std::time::SystemTime` | Query cutoff input. |
| `Promise<[IExtentModel[], number | undefined]>` | `async fn -> Result<(Vec<LegacyExtentModel>, Option<u64>), StorageError>` | Preserve pagination tuple shape. |
| `AsyncIterator<string[]>` | `BoxStream<'_, Vec<String>>` | Batched iteration. |

## Recommended Rust translation
```rust
pub struct LegacyExtentModel {
    pub id: String,
    pub persistency_id: String,
    pub path: String,
    pub size: u64,
    pub last_modify_in_ms: i64,
}

#[async_trait]
pub trait ExtentMetadata: DataStore + Send + Sync {
    async fn update_extent(&mut self, extent: LegacyExtentModel) -> Result<(), StorageError>;
    async fn list_extents(
        &self,
        id: Option<&str>,
        max_results: Option<u64>,
        marker: Option<u64>,
        query_time: Option<chrono::DateTime<chrono::Utc>>,
        unmodified_time: Option<u64>,
    ) -> Result<(Vec<LegacyExtentModel>, Option<u64>), StorageError>;
    fn extent_iterator(&self) -> futures::stream::BoxStream<'_, Vec<String>>;
    async fn delete_extent(&mut self, extent_id: &str) -> Result<(), StorageError>;
    async fn extent_persistency_id(&self, extent_id: &str) -> Result<String, StorageError>;
}
```

## Special handling
- Preserve this file as a distinct legacy contract even though `IExtentMetadataStore` is the actively used newer API.
- The exported model uses the legacy field spellings `persistencyId` and `LastModifyInMS`. Do **not** normalize them away in the porting notes; that would hide future TS diffs.
- The method name `getExtentIterator()` also differs from `IGCExtentProvider.iteratorExtents()`. Keep the difference documented.
- Parameter `UnmodifiedTime` is PascalCase and semantically overlaps with the newer `protectTimeInMs`; record the mismatch rather than collapsing the concepts.

## Change propagation notes
- If TS deletes this legacy interface in favor of `IExtentMetadataStore`, record the removal explicitly instead of silently merging the records.
- Any field rename here must be compared against Loki metadata serialization, which still stores `LastModifyInMS` today.
