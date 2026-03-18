# Porting Record — `src/queue/generated/middleware/end.middleware.ts`

## File info
- Source path: `src/queue/generated/middleware/end.middleware.ts`
- Source lines: `~80`
- Source type: `generated framework`
- Rust target: `azurite-queue/src/generated/middleware/end.rs`
- Crate: `azurite-queue`
- Module: `generated::middleware::end`
- Status: `ported`

## Special handling
Six-stage middleware pipeline component. Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/middleware/end.middleware.md` for detailed fidelity notes.
