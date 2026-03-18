# Porting Record — `src/queue/generated/middleware/dispatch.middleware.ts`

## File info
- Source path: `src/queue/generated/middleware/dispatch.middleware.ts`
- Source lines: `~80`
- Source type: `generated framework`
- Rust target: `azurite-queue/src/generated/middleware/dispatch.rs`
- Crate: `azurite-queue`
- Module: `generated::middleware::dispatch`
- Status: `ported`

## Special handling
Six-stage middleware pipeline component. Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/middleware/dispatch.middleware.md` for detailed fidelity notes.
