# Porting Record — `src/blob/BlobEnvironment.ts`

## File info
- Source path: `src/blob/BlobEnvironment.ts`
- Source lines: `189`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/blob_environment.rs`
- Crate: `azurite-blob`
- Module: `blob_environment`
- Phase: `12.9`
- Status: `not_started`

## Exported API
### Default class `BlobEnvironment implements IBlobEnvironment`
- Module-level `args.option(...)` registration for blob CLI flags.
- Methods mirror the interface getters for blob host/port/keepAlive, runtime flags, TLS settings, persistence, debug logging, and telemetry.

## Dependencies
- `args` CLI parser
- `fs-extra.access/ensureDir`, `path.dirname`
- `IBlobEnvironment`
- blob constants (`DEFAULT_BLOB_SERVER_HOST_NAME`, `DEFAULT_BLOB_LISTENING_PORT`, `DEFAULT_BLOB_KEEP_ALIVE_TIMEOUT`)

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| module-level `args` configuration | one-time CLI parser builder | Preserve the side-effectful registration guard. |
| parsed `flags` bag | struct of optional typed fields | Many getters rely on presence-vs-value semantics. |
| async filesystem validation in getters | async path validation helpers | Errors are thrown lazily when getters are called. |

## Special handling
- `BlobEnvironment.ts:12-75` only registers blob CLI options when `(args as any).config.name` is unset; prior configuration by another environment can suppress this registration.
- `BlobEnvironment.ts:74` sets the parser name to `azurite-blob` after registration.
- `BlobEnvironment.ts:88-90` returns `this.flags.keepAliveTimeout`, not `this.flags.blobKeepAliveTimeout`; this looks like a bug, but it is the current TS behavior.
- `BlobEnvironment.ts:92-97` creates/access-checks the requested location directory lazily in `location()`.
- `BlobEnvironment.ts:99-152` treat boolean flags as presence checks (`!== undefined`), not truthiness of payloads.
- `BlobEnvironment.ts:154-165` rejects `--inMemoryPersistence` together with `--location`, and separately rejects `--extentMemoryLimit` when in-memory persistence is not enabled.
- `BlobEnvironment.ts:172-185` `debug()` returns a path only when the parsed value is a string; bare `--debug` throws `RangeError`.
- `BlobEnvironment.ts:187-188` leaves the default debug return as implicit `undefined`.

## Change propagation notes
- This file duplicates some common-environment semantics but not perfectly. Keep blob-specific quirks like the keep-alive flag typo and lazy directory creation explicit in the Rust porting notes.
- If TS later consolidates CLI parsing with `src/common/Environment.ts`, revisit both records together.
