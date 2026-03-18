# Porting Record — `src/queue/QueueServer.ts`

## File info
- Source path: `src/queue/QueueServer.ts`
- Source lines: `230`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/queue_server.rs`
- Crate: `azurite-queue`
- Module: `queue_server`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `http.Server` | `axum::Server / tokio TcpListener` | HTTP server |
| `Promise<void>` | `async fn` | Server lifecycle |

## Special handling
Queue server lifecycle: initialize → start (bind port) → close. Follows same pattern as blob equivalent. See `porting-db/src/blob/BlobServer.md` for detailed fidelity notes.
