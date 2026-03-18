# Porting Record — `src/queue/generated/handlers/IHandlers.ts`

## File info
- Source path: `src/queue/generated/handlers/IHandlers.ts`
- Source lines: `~50`
- Source type: `generated framework`
- Rust target: `azurite-queue/src/generated/handlers/i_handlers.rs`
- Crate: `azurite-queue`
- Module: `generated::handlers::i_handlers`
- Status: `ported`

## Special handling
Aggregate handler trait dispatching to resource-specific handlers. Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/handlers/IHandlers.md` for detailed fidelity notes.
