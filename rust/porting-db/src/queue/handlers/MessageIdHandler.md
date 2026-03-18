# Porting Record — `src/queue/handlers/MessageIdHandler.ts`

## File info
- Source path: `src/queue/handlers/MessageIdHandler.ts`
- Source lines: `175`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/handlers/message_id_handler.rs`
- Crate: `azurite-queue`
- Module: `handlers::message_id_handler`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueueMetadataStore` | `Arc<dyn IQueueMetadataStore>` | Metadata store |
| `IExtentStore` | `Arc<dyn IExtentStore>` | Extent store |
| `Context` | `&Context` | Request context |
| `Models.*` | `generated models` | Request/response types |

## Special handling
Handles per-message operations: Update (visibility timeout change, message body update) and Delete. Queue-specific — no blob equivalent. Uses pop receipt validation for message identity.
