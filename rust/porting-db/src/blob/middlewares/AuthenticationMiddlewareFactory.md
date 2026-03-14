# Porting Record — `src/blob/middlewares/AuthenticationMiddlewareFactory.ts`

## File info
- Source path: `src/blob/middlewares/AuthenticationMiddlewareFactory.ts`
- Source lines: `60`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/middlewares/authentication_middleware_factory.rs`
- Crate: `azurite-blob`
- Module: `middlewares::authentication_middleware_factory`
- Phase: `12.4`
- Status: `not_started`

## Exported API
### Default class `AuthenticationMiddlewareFactory`
- Constructor: `(logger: ILogger)`
- Methods:
  - `createAuthenticationMiddleware(authenticators)`
  - `authenticate(context, req, res, authenticators)`

## Dependencies
- Common `ILogger`
- Phase 7 `IAuthenticator`
- `BlobStorageContext`, `StorageErrorFactory`
- `ExpressRequestAdapter`, `ExpressResponseAdapter`, generated request/response abstractions
- `DEFAULT_CONTEXT_PATH`

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| middleware factory over authenticator list | middleware builder that iterates boxed authenticators | Preserve ordering and tri-state return handling. |
| `Promise<boolean | undefined>` authenticator contract | `Result<Option<bool>, StorageError>`-style flow | `undefined` means “not applicable”, not “false with error”. |

## Special handling
- `AuthenticationMiddlewareFactory.ts:20-24` rebuilds generated request/response adapters for middleware use instead of sharing raw Express objects.
- `AuthenticationMiddlewareFactory.ts:25-35` collapses the authenticator tri-state into `pass` vs authorization failure; `undefined` and `false` both become `AuthorizationFailure`.
- `AuthenticationMiddlewareFactory.ts:26-27` keeps a TODO noting that public access may eventually need delayed rejection in handlers instead of middleware.
- `AuthenticationMiddlewareFactory.ts:51-58` stops at the first authenticator that returns `true`; it does not collect errors from earlier `undefined` / `false` results.

## Change propagation notes
- Keep authenticator ordering configurable from the caller (`BlobRequestListenerFactory` and `BlobBatchHandler`) rather than hard-coding it here.
- If TS changes the authenticator tri-state contract, update this middleware and every authenticator record together.
