# Porting Record — `src/table/TableServer.ts`

## File info
- Source path: `src/table/TableServer.ts`
- Source lines: `126`
- Source type: `handwritten`
- Rust target: `azurite-table/src/table_server.rs`
- Crate: `azurite-table`
- Module: `table_server`
- Status: `ported`

## Special handling
Table server lifecycle: initialize → start → close. Follows same pattern as blob equivalent. See `porting-db/src/blob/BlobServer.md` for detailed fidelity notes.
