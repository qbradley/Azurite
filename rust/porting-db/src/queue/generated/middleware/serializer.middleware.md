# Porting Record — `src/queue/generated/middleware/serializer.middleware.ts`

## File info
- Source path: `src/queue/generated/middleware/serializer.middleware.ts`
- Source lines: `~80`
- Source type: `generated framework`
- Rust target: `azurite-queue/src/generated/middleware/serializer.rs`
- Crate: `azurite-queue`
- Module: `generated::middleware::serializer`
- Status: `ported`

## Special handling
Six-stage middleware pipeline component. Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/middleware/serializer.middleware.md` for detailed fidelity notes.
