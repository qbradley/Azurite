# Porting Record — `src/table/generated/ExpressRequestAdapter.ts`

## File info
- Source path: `src/table/generated/ExpressRequestAdapter.ts`
- Source lines: `80`
- Source type: `generated framework`
- Rust target: `azurite-table/src/generated/express_request_adapter.rs`
- Crate: `azurite-table`
- Module: `generated::express_request_adapter`
- Status: `ported`

## Special handling
Adapts Express Request to IRequest trait. Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/ExpressRequestAdapter.md` for detailed fidelity notes.
