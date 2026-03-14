# Porting Record — `src/blob/handlers/BlobBatchHandler.ts`

## File info
- Source path: `src/blob/handlers/BlobBatchHandler.ts`
- Source lines: `577`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/handlers/blob_batch_handler.rs`
- Crate: `azurite-blob`
- Module: `handlers::blob_batch_handler`
- Phase: `11.10`
- Status: `not_started`

## Exported API
### Class `BlobBatchHandler`
- Constructor injects account/oauth metadata/extent/logger/loose and optional `disableProductStyle`.
- Public method: `submitBatch()`
- Internal helpers:
  - `streamToBuffer2()`
  - `requestBodyToString()`
  - `getSubRequestOperation()`
  - `parseSubRequests()`
  - `serializeSubResponse()`
  - `HandleOneSubRequest()`
  - `HandleOneFailedRequest()`

## Dependencies
- Rebuilds a mini middleware pipeline from generated blob middleware pieces.
- Instantiates `AppendBlobHandler`, `BlobHandler`, `BlockBlobHandler`, `ContainerHandler`, `PageBlobHandler`, `ServiceHandler`, plus `PageBlobRangesManager`.
- Reuses `AuthenticationMiddlewareFactory` and the normal authenticators (`PublicAccess`, shared-key, account SAS, blob SAS, optional bearer token).
- Uses `BlobBatchSubRequest`, `BlobBatchSubResponse`, and `internalBlobStorageContextMiddleware()`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| ad-hoc middleware arrays of function pointers | explicit vector of boxed closures or dedicated pipeline runner | Order is contract-critical; keep it inspectable. |
| fixed-size batch-body buffering | bounded byte buffer helper | Current implementation hard-limits batch body parsing to 4 MiB. |
| multipart request/response text assembly | literal CRLF/boundary string builder | Preserve header casing, delimiters, and request-boundary reuse. |

## Special handling
- `BlobBatchHandler.ts:62-75` rebuilds blob context for each subrequest by calling the internal blob-context middleware directly, with API-version checks always skipped.
- `BlobBatchHandler.ts:86-130` authenticates subrequests with the same authenticator ordering as the main service pipeline, but there is still a TODO for delayed public-access rejection.
- `BlobBatchHandler.ts:142-185` constructs a fresh handwritten handler set for the batch executor; page/blob handlers share a newly created `PageBlobRangesManager`.
- `BlobBatchHandler.ts:228-241` defines two pipelines: the full execution pipeline (`context → dispatch → auth → deserialize → handler → serialize → end`) and a shorter operation-finder pipeline (`context → dispatch`). There is no strict-model, telemetry, or CORS layer inside batch execution.
- `BlobBatchHandler.ts:281-292` reads the entire batch request into a fixed 4 MiB buffer before parsing.
- `BlobBatchHandler.ts:325-417` parses subrequests by raw string splitting on boundaries and CRLFs. Each subrequest must have `Content-ID`, a request line, and only headers (no request body support today).
- `BlobBatchHandler.ts:376-377` rejects subrequests outside the required path prefix; container-scoped batch requests therefore cannot escape their container.
- `BlobBatchHandler.ts:393-408` currently supports only `Operation.Blob_Delete` and `Operation.Blob_SetTier`, and every subrequest in the batch must target the same operation.
- `BlobBatchHandler.ts:420-449` reuses the request boundary string when serializing the response body instead of generating a fresh response boundary.
- `BlobBatchHandler.ts:473-487` maps most parse failures to a generic `StorageError(400, "InvalidInput", ...)`, but passes through middleware-style storage errors when they already carry the expected fields.
- `BlobBatchHandler.ts:491-498` enforces the Azure max of 256 subrequests.
- `BlobBatchHandler.ts:529-575` uses the generated error middleware to turn both pipeline failures and whole-batch failures into subresponses; this keeps batch error formatting aligned with non-batch requests.

## Change propagation notes
- Batch behavior is tightly coupled to Phase 5 generated middleware and Phase 12 request-listener/auth middleware. If any of those layers move, update this file in lockstep.
- Preserve current limitations (same-operation batches, no subrequest body parsing, 4 MiB parse buffer) until TypeScript changes; these are real current-service constraints.
