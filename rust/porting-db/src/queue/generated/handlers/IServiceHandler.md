# Porting Record — `src/queue/generated/handlers/IServiceHandler.ts`

## File info
- Source path: `src/queue/generated/handlers/IServiceHandler.ts`
- Source lines: `~50`
- Source type: `generated framework`
- Rust target: `azurite-queue/src/generated/handlers/i_service_handler.rs`
- Crate: `azurite-queue`
- Module: `generated::handlers::i_service_handler`
- Status: `ported`

## Special handling
Handler trait for service-level operations (listQueues, getProperties, setProperties, getStats). Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/handlers/IServiceHandler.md` for detailed fidelity notes.
