# Porting Record — `src/queue/middlewares/telemetry.middleware.ts`

## File info
- Source path: `src/queue/middlewares/telemetry.middleware.ts`
- Source lines: `50`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/middlewares/telemetry.rs`
- Crate: `azurite-queue`
- Module: `middlewares::telemetry`
- Status: `ported`

## Special handling
Request/response telemetry logging. Follows same pattern as blob equivalent. See `porting-db/src/blob/middlewares/telemetry.middleware.md` for detailed fidelity notes.
