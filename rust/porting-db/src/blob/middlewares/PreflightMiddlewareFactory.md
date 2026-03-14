# Porting Record — `src/blob/middlewares/PreflightMiddlewareFactory.ts`

## File info
- Source path: `src/blob/middlewares/PreflightMiddlewareFactory.ts`
- Source lines: `465`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/middlewares/preflight_middleware_factory.rs`
- Crate: `azurite-blob`
- Module: `middlewares::preflight_middleware_factory`
- Phase: `12.5`
- Status: `not_started`

## Exported API
### Default class `PreflightMiddlewareFactory`
- Constructor: `(logger: ILogger)`
- Public methods:
  - `createOptionsHandlerMiddleware(metadataStore)`
  - `createCorsRequestMiddleware(metadataStore, blockErrorRequest = false)`
- Internal helpers:
  - `checkOrigin()`
  - `checkMethod()`
  - `checkHeaders()`
  - `getResponseHeaders()`
  - `getExposedHeaders()`

## Dependencies
- Express request/error middleware types
- `glob-to-regexp` for wildcard origin matching
- `@azure/ms-rest-js.Serializer` with generated blob mappers/specifications
- `BlobStorageContext`, `StorageErrorFactory`, `MiddlewareError`, `IBlobMetadataStore`
- Blob constants (`HeaderConstants`, `MethodConstants`, `DEFAULT_CONTEXT_PATH`)

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| mix of normal and error middleware | explicit preflight/cors interceptors for success and error paths | Current Express ordering matters. |
| glob-based CORS origin rules | wildcard matcher over lowercase strings | Preserve comma-splitting and first-match semantics. |
| generated-spec header introspection | helper over generated response metadata snapshots | Used to compute exposed headers for CORS. |

## Special handling
- `PreflightMiddlewareFactory.ts:28-140` only handles preflight requests from the **error** path: it is installed after the serializer and triggered when dispatch/deserialization initially rejected an `OPTIONS` request.
- `PreflightMiddlewareFactory.ts:51-77` requires `Origin` and `Access-Control-Request-Method`; missing or non-string values become `InvalidCorsHeaderValue`.
- `PreflightMiddlewareFactory.ts:82-135` returns the first matching CORS rule, setting `Access-Control-Allow-*` headers and `Access-Control-Allow-Credentials: true`; otherwise it raises `corsPreflightFailure`.
- `PreflightMiddlewareFactory.ts:143-229` installs two variants of the normal CORS middleware: one that wraps error requests (`blockErrorRequest = true`) and one that wraps successful requests.
- `PreflightMiddlewareFactory.ts:183-218` uses first-match CORS selection for normal requests too, and sets `Vary: Origin` whenever non-wildcard origins or any CORS rules are involved.
- `PreflightMiddlewareFactory.ts:246-249` wildcard origins are matched with `glob-to-regexp` against lowercase trimmed strings.
- `PreflightMiddlewareFactory.ts:269-297` wildcard allowed headers only support suffix `*` prefix matching, not arbitrary globs.
- `PreflightMiddlewareFactory.ts:305-381` reconstructs response headers from generated operation specs plus the current `handlerResponses`, explicit Express headers, and error headers. This is more than a simple `res.getHeaders()` wrapper.
- `PreflightMiddlewareFactory.ts:398-403` manually appends `Date`, `Connection`, and `Transfer-Encoding` to the exposed-header candidate set.
- `PreflightMiddlewareFactory.ts:450-460` preserves explicitly configured simple exposed headers even if they were not present on the current response.

## Change propagation notes
- This middleware is tightly coupled to Phase 5 generated `specifications` / `mappers`; if those snapshots change, revisit `getResponseHeaders()`.
- Preserve the current Express installation strategy from `BlobRequestListenerFactory`. Moving this middleware earlier/later changes which requests hit the error-path OPTIONS handler.
