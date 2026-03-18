# Porting Record — `src/queue/handlers/ServiceHandler.ts`

## File info
- Source path: `src/queue/handlers/ServiceHandler.ts`
- Source lines: `265`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/handlers/service_handler.rs`
- Crate: `azurite-queue`
- Module: `handlers::service_handler`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueueMetadataStore` | `Arc<dyn IQueueMetadataStore>` | Metadata store |
| `IExtentStore` | `Arc<dyn IExtentStore>` | Extent store |
| `Context` | `&Context` | Request context |
| `Models.*` | `generated models` | Request/response types |

## Special handling
Handles service-level operations: ListQueues, GetServiceProperties, SetServiceProperties, GetServiceStats. Follows same pattern as blob equivalent. See `porting-db/src/blob/handlers/ServiceHandler.md` for detailed fidelity notes.
