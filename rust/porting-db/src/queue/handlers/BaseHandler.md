# Porting Record — `src/queue/handlers/BaseHandler.ts`

## File info
- Source path: `src/queue/handlers/BaseHandler.ts`
- Source lines: `19`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/handlers/base_handler.rs`
- Crate: `azurite-queue`
- Module: `handlers::base_handler`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueueMetadataStore` | `Arc<dyn IQueueMetadataStore>` | Metadata store |
| `IExtentStore` | `Arc<dyn IExtentStore>` | Extent store |
| `Context` | `&Context` | Request context |
| `Models.*` | `generated models` | Request/response types |

## Special handling
Base handler with shared dependencies (metadataStore, extentStore, logger). Follows same pattern as blob equivalent. See `porting-db/src/blob/handlers/BaseHandler.md` for detailed fidelity notes.
