# Porting Record — `src/blob/main.ts`

## File info
- Source path: `src/blob/main.ts`
- Source lines: `67`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/main.rs`
- Crate: `azurite-blob`
- Module: `main`
- Phase: `12.14`
- Status: `not_started`

## Exported API
### Process entrypoint
- Helper `shutdown(server)`
- Async `main()`
- Top-level `main().catch(...)`

## Dependencies
- Common `Logger`, `setExtentMemoryLimit()`, telemetry client
- `BlobServerFactory`, `BlobEnvironment`
- `BlobServer` / `SqlBlobServer`

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| async process bootstrap | async `main` with signal handlers | Preserve startup/shutdown log order. |
| union server type | enum/trait object | Shutdown helper accepts either blob server implementation. |

## Special handling
- `main.ts:12-21` emits telemetry stop before beginning shutdown and logs to stdout both before and after close.
- `main.ts:27-35` creates the server first, then configures the global singleton logger from `server.config`.
- `main.ts:37-38` constructs a fresh `BlobEnvironment` and calls `setExtentMemoryLimit(env, true)` after `createServer()`, not before.
- `main.ts:41-47` prints startup/listening messages to stdout even though `BlobServer` also logs matching messages in lifecycle hooks.
- `main.ts:49-51` initializes telemetry only after the server has started and after `location()` is resolved from the new environment instance.
- `main.ts:54-61` handles IPC `message === "shutdown"`, `SIGINT`, and `SIGTERM` with one-shot handlers.
- `main.ts:64-67` turns uncaught top-level failures into `Exit due to unhandled error: ...` on stderr and exits with code 1.

## Change propagation notes
- Main/bootstrap ordering matters because logger setup, extent-memory-limit configuration, and telemetry init are not all driven from a single config object. Keep the current sequencing visible.
