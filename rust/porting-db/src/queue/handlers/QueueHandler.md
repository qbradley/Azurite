# Porting Record — `src/queue/handlers/QueueHandler.ts`

## File info
- Source path: `src/queue/handlers/QueueHandler.ts`
- Source lines: `327`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/handlers/queue_handler.rs`
- Crate: `azurite-queue`
- Module: `handlers::queue_handler`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueueMetadataStore` | `Arc<dyn IQueueMetadataStore>` | Metadata store |
| `IExtentStore` | `Arc<dyn IExtentStore>` | Extent store |
| `Context` | `&Context` | Request context |
| `Models.*` | `generated models` | Request/response types |

## Special handling
Handles per-queue operations: Create, Delete, GetProperties, SetMetadata, GetAccessPolicy, SetAccessPolicy. Parallel to blob ContainerHandler pattern.
