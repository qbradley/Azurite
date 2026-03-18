# Porting Record — `src/queue/generated/middleware/deserializer.middleware.ts`

## File info
- Source path: `src/queue/generated/middleware/deserializer.middleware.ts`
- Source lines: `~80`
- Source type: `generated framework`
- Rust target: `azurite-queue/src/generated/middleware/deserializer.rs`
- Crate: `azurite-queue`
- Module: `generated::middleware::deserializer`
- Status: `ported`

## Special handling
Six-stage middleware pipeline component. Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/middleware/deserializer.middleware.md` for detailed fidelity notes.
