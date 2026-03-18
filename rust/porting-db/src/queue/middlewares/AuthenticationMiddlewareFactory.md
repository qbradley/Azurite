# Porting Record — `src/queue/middlewares/AuthenticationMiddlewareFactory.ts`

## File info
- Source path: `src/queue/middlewares/AuthenticationMiddlewareFactory.ts`
- Source lines: `80`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/middlewares/authentication_middleware_factory.rs`
- Crate: `azurite-queue`
- Module: `middlewares::authentication_middleware_factory`
- Status: `ported`

## Special handling
Authentication middleware that chains authenticators (SharedKey → SAS → Token). Follows same pattern as blob equivalent. See `porting-db/src/blob/middlewares/AuthenticationMiddlewareFactory.md` for detailed fidelity notes.
