# Porting Record — `src/table/generated/ExpressResponseAdapter.ts`

## File info
- Source path: `src/table/generated/ExpressResponseAdapter.ts`
- Source lines: `90`
- Source type: `generated framework`
- Rust target: `azurite-table/src/generated/express_response_adapter.rs`
- Crate: `azurite-table`
- Module: `generated::express_response_adapter`
- Status: `ported`

## Special handling
Adapts IResponse to Express Response. Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/ExpressResponseAdapter.md` for detailed fidelity notes.
