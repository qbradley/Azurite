# Porting Record — `src/queue/generated/ExpressMiddlewareFactory.ts`

## File info
- Source path: `src/queue/generated/ExpressMiddlewareFactory.ts`
- Source lines: `50`
- Source type: `generated framework`
- Rust target: `azurite-queue/src/generated/express_middleware_factory.rs`
- Crate: `azurite-queue`
- Module: `generated::express_middleware_factory`
- Status: `ported`

## Special handling
Express.js → axum adapter for middleware. Follows same pattern as blob equivalent. See `porting-db/src/blob/generated/ExpressMiddlewareFactory.md` for detailed fidelity notes.
