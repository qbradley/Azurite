# Porting Record — `src/blob/BlobRequestListenerFactory.ts`

## File info
- Source path: `src/blob/BlobRequestListenerFactory.ts`
- Source lines: `227`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/blob_request_listener_factory.rs`
- Crate: `azurite-blob`
- Module: `blob_request_listener_factory`
- Phase: `12.11`
- Status: `not_started`

## Exported API
### Default class `BlobRequestListenerFactory implements IRequestListenerFactory`
- Constructor injects metadata/extent/account stores plus access-log, mode, auth, and URL-style options.
- Method: `createRequestListener()` returns the configured Express app.

## Dependencies
- Express + `morgan`
- Common `IRequestListenerFactory`, `ServerBase.RequestListener`, singleton `logger`
- Phase 5 generated middleware factory and handler interface bundle
- Phase 11 handwritten handlers and `PageBlobRangesManager`
- Phase 12 middleware factories (`Authentication`, `Preflight`, `StrictModel`, `telemetry`, `blobStorageContext`)
- Phase 7 authenticators and optional OAuth level

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| Express app assembly | axum/tower router + middleware stack builder | Preserve current ordering exactly. |
| shared handler bundle object | struct of concrete handler instances | Generated handler middleware expects named handler fields. |
| singleton logger imported from module | shared global/logger handle | Main/bootstrap code configures this singleton separately. |

## Special handling
- `BlobRequestListenerFactory.ts:63` disables Express's `x-powered-by` header immediately.
- `BlobRequestListenerFactory.ts:72-120` constructs one shared `PageBlobRangesManager` and injects it into both `BlobHandler` and `PageBlobHandler`.
- `BlobRequestListenerFactory.ts:140-223` installs middleware in a strict order:
  1. access log (`morgan`) when enabled
  2. blob storage context middleware
  3. generated dispatch middleware
  4. strict-model middleware (only when `loose` is false/undefined)
  5. authentication middleware
  6. generated deserializer
  7. generated handler middleware
  8. CORS error-path middleware (`blockErrorRequest = true`)
  9. CORS success-path middleware (`blockErrorRequest = false`)
  10. generated serializer
  11. OPTIONS preflight error middleware
  12. generated error middleware
  13. telemetry middleware
  14. generated end middleware
- `BlobRequestListenerFactory.ts:160-178` orders authenticators as public access, shared key, account SAS, blob SAS, then optional bearer token.
- `BlobRequestListenerFactory.ts:152-154` strict mode is enabled unless `loose === true`; `undefined` behaves like strict mode.

## Change propagation notes
- The middleware order here is architecture-critical for Aragorn. Reordering can silently change whether requests hit strict mode, auth, CORS, serializer, or the OPTIONS error path.
- Batch handling in `BlobBatchHandler` reconstructs a partial shadow of this pipeline; if TS changes one, audit the other.
