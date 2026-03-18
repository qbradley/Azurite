# Porting Record — `src/queue/QueueRequestListenerFactory.ts`

## File info
- Source path: `src/queue/QueueRequestListenerFactory.ts`
- Source lines: `176`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/queue_request_listener_factory.rs`
- Crate: `azurite-queue`
- Module: `queue_request_listener_factory`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `express.Application` | `axum::Router` | HTTP router |
| `IRequestListenerFactory` | `trait impl` | Factory pattern |

## Special handling
Creates the HTTP request listener (Express app → axum Router) with all middleware and handler wiring. Follows same pattern as blob equivalent. See `porting-db/src/blob/BlobRequestListenerFactory.md` for detailed fidelity notes.
