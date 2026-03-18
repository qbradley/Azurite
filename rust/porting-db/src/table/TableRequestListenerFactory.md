# Porting Record — `src/table/TableRequestListenerFactory.ts`

## File info
- Source path: `src/table/TableRequestListenerFactory.ts`
- Source lines: `194`
- Source type: `handwritten`
- Rust target: `azurite-table/src/table_request_listener_factory.rs`
- Crate: `azurite-table`
- Module: `table_request_listener_factory`
- Status: `ported`

## Special handling
Creates HTTP request listener with table middleware and handler wiring. Follows same pattern as blob equivalent. See `porting-db/src/blob/BlobRequestListenerFactory.md` for detailed fidelity notes.
