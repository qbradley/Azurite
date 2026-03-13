# Porting Record — `src/common/IEnvironment.ts`

## File info
- Source path: `src/common/IEnvironment.ts`
- Source lines: `8`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/i_environment.rs`
- Crate: `azurite-common`
- Module: `i_environment`
- Phase: `1.11`
- Status: `ported`

## Exported API
### Default interface `IEnvironment`
- No own members.
- Extends `IBlobEnvironment`, `IQueueEnvironment`, and `ITableEnvironment`.

### Inherited member groups that matter to the Rust port
- Blob-specific: `blobHost()`, `blobPort()`, `blobKeepAliveTimeout()`, `oauth()`.
- Queue-specific: `queueHost()`, `queuePort()`, `queueKeepAliveTimeout()`.
- Table-specific: `tableHost()`, `tablePort()`, `tableKeepAliveTimeout()`.
- Shared methods repeated across the service interfaces: `location()`, `silent()`, `loose()`, `skipApiVersionCheck()`, `disableProductStyleUrl()`, `debug()`, `inMemoryPersistence()`, `disableTelemetry()`.

## Dependencies
- `../blob/IBlobEnvironment`
- `../queue/IQueueEnvironment`
- `../table/ITableEnvironment`
- Concrete implementations: `Environment`, `VSCEnvironment`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| interface aggregation | supertrait or flattened config struct | Pure composition, no new members. |
| `Promise<string>` | `async fn -> Result<String, StorageError>` | Used for location discovery. |
| `Promise<string | boolean | undefined>` | `async fn -> Result<Option<DebugTarget>, StorageError>` | Consider enum for path-vs-boolean debug setting if you flatten config. |

## Recommended Rust translation
```rust
pub trait Environment: BlobEnvironment + QueueEnvironment + TableEnvironment {}
impl<T> Environment for T where T: BlobEnvironment + QueueEnvironment + TableEnvironment {}
```

## Special handling
- This is a pure aggregation interface. The tricky part is that the three superinterfaces repeat method names with identical signatures.
- If the Rust port needs trait objects, a marker supertrait may be awkward because duplicate method names are hard to call through `dyn Environment`. In that case, prefer a concrete flattened `EnvironmentConfig` struct or an explicitly flattened trait.
- `oauth()` arrives only from `IBlobEnvironment`, but `Environment` and `VSCEnvironment` expose it globally and non-blob code reads it through common configuration flow. Preserve that cross-service visibility.

## Change propagation notes
- Any method added to one service-specific environment interface can affect this aggregate type immediately.
- If TS ever stops sharing methods like `location()` or `debug()` across services, revisit the Rust aggregation strategy rather than assuming a single flattened config still fits.

## Rust port notes
- Ported to `rust/crates/azurite-common/src/i_environment.rs` as a flattened `IEnvironment` trait covering the union of blob, queue, and table environment members.
- Forced deviation: `azurite-common` cannot depend on the service crates that will eventually host `IBlobEnvironment`, `IQueueEnvironment`, and `ITableEnvironment`, so Phase 1 keeps one local aggregate trait plus `DebugValue` to model `string | boolean | undefined`.
