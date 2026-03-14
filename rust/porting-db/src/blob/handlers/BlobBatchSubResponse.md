# Porting Record — `src/blob/handlers/BlobBatchSubResponse.ts`

## File info
- Source path: `src/blob/handlers/BlobBatchSubResponse.ts`
- Source lines: `84`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/handlers/blob_batch_sub_response.rs`
- Crate: `azurite-blob`
- Module: `handlers::blob_batch_sub_response`
- Phase: `11.12`
- Status: `not_started`

## Exported API
### Class `BlobBatchSubResponse implements IResponse`
- Constructor carries optional `content_id` and `protocolWithVersion`.
- Methods implement the generated response contract: status/header setters/getters, `getBodyStream()`, `getBodyContent()`, `end()`.

## Dependencies
- Node `STATUS_CODES`.
- `SubResponseTextBodyStream` accumulator.
- Generated `IResponse`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| in-memory response collector | concrete response shim storing status, headers, and body text | Used only by batch execution. |
| `string | string[] | number | boolean` header setter | header enum/value adapter that stringifies numbers and booleans | Match the generated adapter behavior. |

## Special handling
- `BlobBatchSubResponse.ts:41-47` stringifies numeric and boolean header values before storage.
- `BlobBatchSubResponse.ts:49-53` only stores string/array header values and intentionally keeps the guard because of a noted TypeScript bug.
- `BlobBatchSubResponse.ts:64-66` always reports `headersSent()` as `false`.
- `BlobBatchSubResponse.ts:81-83` finalizes the status message lazily during `end()` using Node's `STATUS_CODES`, defaulting to `'unknown'`.

## Change propagation notes
- If the generated `IResponse` contract changes, this batch shim and `SubResponseTextBodyStream` need to stay aligned.
- Preserve lazy status-message filling; batch serialization expects it to happen at end time, not when the status code is first set.
