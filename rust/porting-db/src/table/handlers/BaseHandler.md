# Porting Record — `src/table/handlers/BaseHandler.ts`

## File info
- Source path: `src/table/handlers/BaseHandler.ts`
- Source lines: `17`
- Source type: `handwritten`
- Rust target: `azurite-table/src/handlers/base_handler.rs`
- Crate: `azurite-table`
- Module: `handlers::base_handler`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `ITableMetadataStore` | `Arc<dyn ITableMetadataStore>` | Metadata store |
| `Context` | `&Context` | Request context |
| `Models.*` | `generated models` | Request/response types |

## Special handling
Follows same pattern as blob equivalent. See `porting-db/src/blob/handlers/BaseHandler.md` for detailed fidelity notes.
