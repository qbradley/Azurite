# Porting Record — `src/blob/middlewares/StrictModelMiddlewareFactory.ts`

## File info
- Source path: `src/blob/middlewares/StrictModelMiddlewareFactory.ts`
- Source lines: `74`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/middlewares/strict_model_middleware_factory.rs`
- Crate: `azurite-blob`
- Module: `middlewares::strict_model_middleware_factory`
- Phase: `12.6`
- Status: `not_started`

## Exported API
### Types and validators
- `StrictModelRequestValidator`
- `UnsupportedHeadersBlocker`
- `UnsupportedParametersBlocker`
### Default class `StrictModelMiddlewareFactory`
- Constructor: `(logger, validators)`
- Methods: `createStrictModelMiddleware()`, private `validate()`

## Dependencies
- Common `ILogger`
- `BlobStorageContext`, `StrictModelNotSupportedError`
- Generated `Context` / `IRequest` abstractions and `ExpressRequestAdapter`
- Blob constants (`DEFAULT_CONTEXT_PATH`, `HeaderConstants`)

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| async validator function list | vector of async validator fns/traits | Preserve sequential execution order. |
| strict-mode middleware over adapted request | request pre-validator layer | Runs before auth in the main listener pipeline. |

## Special handling
- `StrictModelMiddlewareFactory.ts:17-36` blocks specific unsupported headers (`x-ms-content-crc64`, `x-ms-range-get-content-crc64`, encryption key headers) by raising `StrictModelNotSupportedError`.
- `StrictModelMiddlewareFactory.ts:38-52` defines an empty unsupported-parameter list today; the validator is present but currently no-ops.
- `StrictModelMiddlewareFactory.ts:60-65` uses `.then(next).catch(next)`, so successful validation simply calls `next(undefined)`.
- `StrictModelMiddlewareFactory.ts:68-72` reconstructs a fresh `BlobStorageContext` plus `ExpressRequestAdapter` for every validation pass.

## Change propagation notes
- If TS adds unsupported query parameters or more headers, update the validator arrays rather than inventing a different strict-mode architecture in Rust.
- Keep the empty-parameters validator visible even though it does nothing today; it signals an intended extension seam.
