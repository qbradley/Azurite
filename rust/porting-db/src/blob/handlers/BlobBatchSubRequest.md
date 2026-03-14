# Porting Record — `src/blob/handlers/BlobBatchSubRequest.ts`

## File info
- Source path: `src/blob/handlers/BlobBatchSubRequest.ts`
- Source lines: `82`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/handlers/blob_batch_sub_request.rs`
- Crate: `azurite-blob`
- Module: `handlers::blob_batch_sub_request`
- Phase: `11.11`
- Status: `not_started`

## Exported API
### Class `BlobBatchSubRequest implements IRequest`
- Constructor carries `content_id`, `url`, `method`, `protocolWithVersion`, and a mutable lowercase-header map.
- Methods mirror the generated `IRequest` contract: method/url/path/header/query/protocol getters plus `setHeader()`.

## Dependencies
- `URLBuilder` from `@azure/ms-rest-js`.
- `Readable.from([])` to satisfy the no-body stream contract.
- Generated `IRequest` / `HttpMethod`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| request adapter object over parsed batch text | concrete struct implementing the local request trait | This is not an HTTP client request; it is an in-memory shim for middleware execution. |
| lowercase mutable header bag | `HashMap<String, HeaderValueLike>` normalized to lowercase on set/get | Preserve lowercasing and missing raw-header fidelity. |

## Special handling
- `BlobBatchSubRequest.ts:25-28` derives `endpoint` from the parsed URL scheme and host.
- `BlobBatchSubRequest.ts:36-45` has no real body support: `getBodyStream()` returns an empty readable stream, `getBody()` always returns `undefined`, and `setBody()` is a no-op.
- `BlobBatchSubRequest.ts:48-59` throws `TypeError` when `getHeader()` is called with a missing or non-string field.
- `BlobBatchSubRequest.ts:66-68` returns an empty raw-header array; metadata-preserving paths cannot recover original header casing here.
- `BlobBatchSubRequest.ts:79-80` lowercases header keys on write.

## Change propagation notes
- If TS adds real batch subrequest bodies later, this file and `BlobBatchHandler.parseSubRequests()` must change together.
- Keep header normalization behavior aligned with TypeScript; some middleware looks up lowercase literal names.
