# Porting Record — `src/queue/middlewares/PreflightMiddlewareFactory.ts`

## File info
- Source path: `src/queue/middlewares/PreflightMiddlewareFactory.ts`
- Source lines: `40`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/middlewares/preflight_middleware_factory.rs`
- Crate: `azurite-queue`
- Module: `middlewares::preflight_middleware_factory`
- Status: `ported`

## Special handling
CORS preflight request handler. Follows same pattern as blob equivalent. See `porting-db/src/blob/middlewares/PreflightMiddlewareFactory.md` for detailed fidelity notes.
