# Porting Record — `src/blob/BlobServerFactory.ts`

## File info
- Source path: `src/blob/BlobServerFactory.ts`
- Source lines: `104`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/blob_server_factory.rs`
- Crate: `azurite-blob`
- Module: `blob_server_factory`
- Phase: `12.13`
- Status: `not_started`

## Exported API
### Class `BlobServerFactory`
- Method: `createServer(blobEnvironment?: IBlobEnvironment): Promise<BlobServer | SqlBlobServer>`

## Dependencies
- `BlobEnvironment`, `IBlobEnvironment`
- `BlobConfiguration`, `SqlBlobConfiguration`
- `BlobServer`, `SqlBlobServer`
- common `DEFAULT_SQL_OPTIONS`
- blob constants for DB paths and mutable persistence array

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| async server factory returning one of two concrete server types | enum/trait-object factory result | The SQL-vs-Loki split is runtime-selected. |
| environment-driven config build | explicit config builder | Preserve lazy environment getter calls and validation timing. |
| mutable module-level defaults | shared mutable config seed | This file mutates imported constants before server construction. |

## Special handling
- `BlobServerFactory.ts:21-23` hard-codes `isVSC = false`; the VS Code path remains unimplemented.
- `BlobServerFactory.ts:27-33` still checks `typeof debugFilePath === "boolean"` even though the current `BlobEnvironment.debug()` returns `string | undefined` or throws.
- `BlobServerFactory.ts:35-38` mutates `DEFAULT_BLOB_PERSISTENCE_ARRAY[0].locationPath` in place for the resolved workspace location.
- `BlobServerFactory.ts:41-43` chooses SQL mode solely from `process.env.AZURITE_DB`.
- `BlobServerFactory.ts:45-50` rejects `--inMemoryPersistence` and `--extentMemoryLimit` when SQL metadata storage is active.
- `BlobServerFactory.ts:52-72` and `74-95` build separate SQL and Loki/in-memory configuration objects but keep most flags aligned.
- `BlobServerFactory.ts:97-102` throws `Not implemented.` for the currently dead VS Code branch.

## Change propagation notes
- Keep global mutation of `DEFAULT_BLOB_PERSISTENCE_ARRAY` visible in the Rust porting notes; changing it to a purely local config clone could mask future TS changes.
- If TS ever enables the VS Code path, revisit this whole factory and the environment abstraction together.
