# Porting Record — `src/blob/IBlobEnvironment.ts`

## File info
- Source path: `src/blob/IBlobEnvironment.ts`
- Source lines: `18`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/i_blob_environment.rs`
- Crate: `azurite-blob`
- Module: `i_blob_environment`
- Phase: `12.8`
- Status: `not_started`

## Exported API
### Default interface `IBlobEnvironment`
- Blob host/port/keepAlive getters
- Shared runtime getters: `location()`, `silent()`, `loose()`, `skipApiVersionCheck()`, `cert()`, `key()`, `pwd()`, `debug()`, `oauth()`, `disableProductStyleUrl()`, `inMemoryPersistence()`, `extentMemoryLimit()`, `disableTelemetry()`

## Dependencies
- Conceptually extends the common environment surface but is defined locally in blob service.
- Implemented by `BlobEnvironment` and consumed by `BlobServerFactory` / `main.ts`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| local service environment trait | service-specific Rust trait or config adapter | Keep blob-specific getters explicit. |
| `Promise<string | boolean | undefined>` debug getter | async fn returning optional path / error | The interface is broader than the current concrete implementation. |

## Special handling
- `IBlobEnvironment.ts:12` allows `debug(): Promise<string | boolean | undefined>`, but `BlobEnvironment` currently returns only `string | undefined` or throws; keep the interface/implementation asymmetry visible.
- This interface intentionally duplicates much of the common environment surface rather than importing the common trait directly.

## Change propagation notes
- If `Environment.ts` / common environment getters change, audit this blob-specific interface and `BlobServerFactory` together.
