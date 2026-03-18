# Porting Record — `src/queue/generated/ExpressRequestAdapter.ts`

## File info
- Source path: `src/queue/generated/ExpressRequestAdapter.ts`
- Source lines: `80`
- Source type: `generated framework`
- Rust target: `azurite-queue/src/generated/express_request_adapter.rs`
- Crate: `azurite-queue`
- Module: `generated::express_request_adapter`
- Status: `ported`

## Special handling
Adapts Express Request to IRequest trait. Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/ExpressRequestAdapter.md` for detailed fidelity notes.
