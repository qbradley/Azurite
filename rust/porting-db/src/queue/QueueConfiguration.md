# Porting Record — `src/queue/QueueConfiguration.ts`

## File info
- Source path: `src/queue/QueueConfiguration.ts`
- Source lines: `66`
- Source type: `handwritten class`
- Rust target (per `PORTING-ORDER.md`): `azurite-queue/src/queue_configuration.rs`
- Crate: `azurite-queue`
- Module: `queue_configuration`
- Phase: `14.18`
- Status: `ported`

## Exported API
### Default class `QueueConfiguration`
- Extends `ConfigurationBase` (Phase 4)
- Constructor: `new QueueConfiguration(host?, port?, keepAliveTimeout?, metadataDBPath?, extentDBPath?, persistencePathArray?, enableAccessLog?, accessLogWriteStream?, enableDebugLog?, debugLogFilePath?, loose?, skipApiVersionCheck?, cert?, key?, pwd?, oauth?, disableProductStyleUrl?, isMemoryPersistence?, memoryStore?)`
- Public properties (readonly):
  - `metadataDBPath: string`
  - `extentDBPath: string`
  - `persistencePathArray: StoreDestinationArray`
  - `isMemoryPersistence: boolean`
  - `memoryStore?: MemoryExtentChunkStore`

## Dependencies
- `ConfigurationBase` (Phase 4): Parent class for common HTTP/logging/SSL configuration
- `StoreDestinationArray` (Phase 1): Type alias for persistence backend paths
- `MemoryExtentChunkStore` (Phase 2): In-memory extent storage implementation
- Queue-specific constants: `DEFAULT_QUEUE_SERVER_HOST_NAME`, `DEFAULT_QUEUE_LISTENING_PORT`, `DEFAULT_QUEUE_KEEP_ALIVE_TIMEOUT`, `DEFAULT_QUEUE_LOKI_DB_PATH`, `DEFAULT_QUEUE_EXTENT_LOKI_DB_PATH`, `DEFAULT_QUEUE_PERSISTENCE_ARRAY`, `DEFAULT_ENABLE_ACCESS_LOG`, `DEFAULT_ENABLE_DEBUG_LOG`

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `class extends ConfigurationBase` | `pub struct` or composition | Extend ConfigurationBase trait or embed as field |
| `public readonly` properties | `pub` struct fields | Direct mapping |
| Optional constructor parameters with defaults | Rust builder pattern or Option<T> | Use defaults from constants or optional fields |
| `NodeJS.WritableStream` | `Box<dyn Write>` or custom trait | For access/debug log streams |
| `MemoryExtentChunkStore` | `Arc<RwLock<MemoryExtentChunkStore>>` or owned | If in-memory mode, store reference to shared instance |

## Special handling
1. **Constructor parameter defaults** (`QueueConfiguration.ts:28-48`)
   - All parameters have defaults from imported constants
   - Queue-specific defaults: `metadataDBPath`, `extentDBPath`, `persistencePathArray`
   - Preserve exact default values for compatibility

2. **Inheritance from ConfigurationBase**
   - Passes 13 parameters to parent constructor (host, port, keepAliveTimeout, logging, SSL, loose/API checks, oauth, disableProductStyleUrl)
   - Retains 5 queue-specific readonly properties
   - Must preserve parent initialization order for Phase 4 compatibility

3. **Memory persistence coupling**
   - `isMemoryPersistence: boolean` controls whether Loki DB is persisted to disk
   - `memoryStore?: MemoryExtentChunkStore` is only populated in memory mode
   - Loki DB path fields are always set but only used when `isMemoryPersistence === false`

4. **Path array strategy**
   - `persistencePathArray: StoreDestinationArray` supports multiple backend destinations
   - Default is `DEFAULT_QUEUE_PERSISTENCE_ARRAY` (typically single Loki DB + fallback)
   - See Phase 1 IExtentStore for array semantics

## Change propagation notes
- If `ConfigurationBase` adds new fields/parameters, QueueConfiguration must pass them through
- If LokiJS is replaced with another persistence backend, queue-specific path field names should remain stable
- The readonly property pattern ensures immutability after construction — preserve in Rust
- Match blob-layer `BlobConfiguration` structure for consistency (Phase 12 equivalent)

