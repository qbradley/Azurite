# Porting Record — `src/blob/BlobServer.ts`

## File info
- Source path: `src/blob/BlobServer.ts`
- Source lines: `230`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/blob_server.rs`
- Crate: `azurite-blob`
- Module: `blob_server`
- Phase: `12.12`
- Status: `not_started`

## Exported API
### Default class `BlobServer extends ServerBase implements ICleaner`
- Constructor builds HTTP/HTTPS server, metadata stores, extent stores, account store, request-listener factory, and GC manager.
- Public method: `clean()`
- Lifecycle hooks overriding `ServerBase`: `beforeStart()`, `afterStart()`, `beforeClose()`, `afterClose()`

## Dependencies
- Node `http` / `https`
- Common `AccountDataStore`, `ServerBase`, `FSExtentStore`, `MemoryExtentStore`, `LokiExtentMetadataStore`, config cert helpers, `logger`
- Blob `BlobConfiguration`, `BlobRequestListenerFactory`, `BlobGCManager`, `LokiBlobMetadataStore`, `StorageError`

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| server subclass with lifecycle hooks | concrete service struct wrapping common server base | Preserve start/close hook ordering. |
| runtime-selected HTTP vs HTTPS server | enum/configured listener type | Cert handling is delegated to config and selected at construction. |
| FS vs memory extent store swap | enum-backed storage backend composition | Keep the backend choice at construction time. |

## Special handling
- `BlobServer.ts:54-72` chooses HTTPS when `configuration.hasCert()` reports `PEM` or `PFX`; otherwise it creates plain HTTP.
- `BlobServer.ts:77-97` always uses `LokiBlobMetadataStore` for metadata, but swaps between `MemoryExtentStore` and `FSExtentStore` for extents based on `isMemoryPersistence`.
- `BlobServer.ts:87-93` passes a `StorageError` constructor callback into `MemoryExtentStore`, preserving blob-specific error construction inside the common store.
- `BlobServer.ts:118-135` installs a `BlobGCManager` whose critical-error callback logs to both console and logger, then asynchronously closes the server.
- `BlobServer.ts:151-170` only allows `clean()` when the server status is already `Closed`.
- `BlobServer.ts:177-195` starts dependencies in the order account store → blob metadata store → extent metadata store → extent store → GC manager.
- `BlobServer.ts:185-187` guards `extentMetadataStore.init()` with `if (this.metadataStore !== undefined)` instead of checking `extentMetadataStore`; this typo is part of current TS source.
- `BlobServer.ts:207-229` closes components in the order GC manager → extent store → extent metadata store → blob metadata store → account store.

## Change propagation notes
- Preserve the constructor-time assembly seams: metadata backend, extent backend, request-listener factory, and GC manager are all swappable in TypeScript.
- If TS changes lifecycle ordering, port it literally; startup/shutdown timing affects persistence and GC behavior.
