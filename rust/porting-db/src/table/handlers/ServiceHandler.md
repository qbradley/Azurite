# Porting Record — `src/table/handlers/ServiceHandler.ts`

## File info
- Source path: `src/table/handlers/ServiceHandler.ts`
- Source lines: `153`
- Source type: `handwritten`
- Rust target: `azurite-table/src/handlers/service_handler.rs`
- Crate: `azurite-table`
- Module: `handlers::service_handler`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `ITableMetadataStore` | `Arc<dyn ITableMetadataStore>` | Metadata store |
| `Context` | `&Context` | Request context |
| `Models.*` | `generated models` | Request/response types |

## Special handling
Service-level operations: GetServiceProperties, SetServiceProperties, GetServiceStats. Follows same pattern as blob equivalent. See `porting-db/src/blob/handlers/ServiceHandler.md` for detailed fidelity notes.
