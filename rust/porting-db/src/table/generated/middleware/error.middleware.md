# Porting Record — `src/table/generated/middleware/error.middleware.ts`

## File info
- Source path: `src/table/generated/middleware/error.middleware.ts`
- Source lines: `~80`
- Source type: `generated framework`
- Rust target: `azurite-table/src/generated/middleware/error.rs`
- Crate: `azurite-table`
- Module: `generated::middleware::error`
- Status: `ported`

## Special handling
Six-stage middleware pipeline component. Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/middleware/error.middleware.md` for detailed fidelity notes.
