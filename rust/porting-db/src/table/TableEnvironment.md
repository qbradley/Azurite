# Porting Record — `src/table/TableEnvironment.ts`

## File info
- Source path: `src/table/TableEnvironment.ts`
- Source lines: `170`
- Source type: `handwritten`
- Rust target: `azurite-table/src/table_environment.rs`
- Crate: `azurite-table`
- Module: `table_environment`
- Status: `ported`

## Special handling
Table environment implementation. Parses CLI args and env vars for table service config. Follows same pattern as blob equivalent. See `porting-db/src/blob/BlobEnvironment.md` for detailed fidelity notes.
