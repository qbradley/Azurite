# Porting Record — `src/queue/generated/handlers/handlerMappers.ts`

## File info
- Source path: `src/queue/generated/handlers/handlerMappers.ts`
- Source lines: `~50`
- Source type: `generated framework`
- Rust target: `azurite-queue/src/generated/handlers/handler_mappers.rs`
- Crate: `azurite-queue`
- Module: `generated::handlers::handler_mappers`
- Status: `ported`

## Special handling
Maps operation enum to handler method dispatch. Index-coupled to operation enum — reordering breaks dispatch.. Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/handlers/handlerMappers.md` for detailed fidelity notes.
