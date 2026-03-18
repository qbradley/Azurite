# Porting Record — `src/table/middleware/AuthenticationMiddlewareFactory.ts`

## File info
- Source path: `src/table/middleware/AuthenticationMiddlewareFactory.ts`
- Source lines: `80`
- Source type: `handwritten`
- Rust target: `azurite-table/src/middlewares/authentication_middleware_factory.rs`
- Crate: `azurite-table`
- Module: `middlewares::authentication_middleware_factory`
- Status: `ported`

## Special handling
Authentication middleware chain. Table adds SharedKeyLite authenticator not present in blob/queue. Follows same pattern as blob equivalent. See `porting-db/src/blob/middlewares/AuthenticationMiddlewareFactory.md` for detailed fidelity notes.
