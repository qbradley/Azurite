# Porting Record — `src/queue/handlers/MessagesHandler.ts`

## File info
- Source path: `src/queue/handlers/MessagesHandler.ts`
- Source lines: `382`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/handlers/messages_handler.rs`
- Crate: `azurite-queue`
- Module: `handlers::messages_handler`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueueMetadataStore` | `Arc<dyn IQueueMetadataStore>` | Metadata store |
| `IExtentStore` | `Arc<dyn IExtentStore>` | Extent store |
| `Context` | `&Context` | Request context |
| `Models.*` | `generated models` | Request/response types |

## Special handling
Handles message collection operations: Enqueue (put), Dequeue (get with visibility timeout), Peek (read without dequeue), Clear (delete all). Queue-specific — no blob equivalent. Dequeue involves atomic visibility timeout update + dequeue count increment.
