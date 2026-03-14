# Porting Record — `src/blob/BlobConfiguration.ts`

## File info
- Source path: `src/blob/BlobConfiguration.ts`
- Source lines: `66`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/blob_configuration.rs`
- Crate: `azurite-blob`
- Module: `blob_configuration`
- Phase: `12.10`
- Status: `not_started`

## Exported API
### Default class `BlobConfiguration extends ConfigurationBase`
- Constructor extends the common configuration with blob-specific metadata DB path, extent DB path, persistence locations, in-memory flag, and optional `MemoryExtentChunkStore`.

## Dependencies
- `ConfigurationBase`
- `StoreDestinationArray`, `MemoryExtentChunkStore`
- blob defaults from `./utils/constants`

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| subclass adding readonly blob-specific fields | config struct composing common config plus blob extras | Keep constructor parameter order near-1:1 for propagation. |
| optional shared in-memory chunk store | optional Arc/shared store handle | Only used for in-memory persistence mode. |

## Special handling
- `BlobConfiguration.ts:27-48` mostly forwards arguments to `ConfigurationBase` but also exposes `metadataDBPath`, `extentDBPath`, `persistencePathArray`, `isMemoryPersistence`, and `memoryStore` as blob-specific public fields.
- `BlobConfiguration.ts:16-23` documents that alternate blob server implementations should create new configuration subclasses rather than mutating this class.

## Change propagation notes
- This file is intentionally thin. If TS adds new blob-only configuration knobs, keep them here rather than leaking them into the common config layer unless TypeScript does the same.
