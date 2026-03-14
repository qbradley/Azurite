# Porting Record — `src/blob/middlewares/telemetry.middleware.ts`

## File info
- Source path: `src/blob/middlewares/telemetry.middleware.ts`
- Source lines: `49`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/middlewares/telemetry.rs`
- Crate: `azurite-blob`
- Module: `middlewares::telemetry`
- Phase: `12.7`
- Status: `not_started`

## Exported API
### Functions/classes
- Internal helper `telemetryMiddleware(context, next)`
- Default class `TelemetryMiddlewareFactory`
  - constructor `(contextPath = "default_context")`
  - `createTelemetryMiddleware()`

## Dependencies
- `AzuriteTelemetryClient.TraceRequest()`
- Generated `Context`, `MiddlewareFactory.NextFunction`, `ExpressRequestAdapter`, `ExpressResponseAdapter`
- Express `RequestHandler`

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| tiny side-effect middleware | request-observer layer | No branching logic; just emit telemetry then continue. |
| context-path default string | plain constructor field | Request-listener code overrides the default. |

## Special handling
- `telemetry.middleware.ts:19-26` simply traces the request and then calls `next()`; it does not inspect handler success/failure.
- `telemetry.middleware.ts:31` defaults `contextPath` to `"default_context"`, but `BlobRequestListenerFactory.ts:126-127` overrides it with `DEFAULT_CONTEXT_PATH`.
- `telemetry.middleware.ts:40-46` rebuilds generated request/response adapters solely to construct a telemetry `Context`.

## Change propagation notes
- Keep this middleware late in the pipeline, matching `BlobRequestListenerFactory`, so telemetry sees the final context/error state TypeScript currently observes.
