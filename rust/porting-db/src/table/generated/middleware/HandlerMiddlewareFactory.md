# Porting Record — `src/table/generated/middleware/HandlerMiddlewareFactory.ts`

## File info
- Source path: `src/table/generated/middleware/HandlerMiddlewareFactory.ts`
- Source lines: `~80`
- Source type: `generated framework`
- Rust target: `azurite-table/src/generated/middleware/handler_middleware_factory.rs`
- Crate: `azurite-table`
- Module: `generated::middleware::handler_middleware_factory`
- Status: `ported`

## Special handling
Six-stage middleware pipeline component. Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/middleware/HandlerMiddlewareFactory.md` for detailed fidelity notes.
