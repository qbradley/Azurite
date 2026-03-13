# Porting Record — `src/blob/errors/StorageError.ts`

## File info
- Source path: `src/blob/errors/StorageError.ts`
- Source lines: `65`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/errors/storage_error.rs`
- Crate: `azurite-blob`
- Module: `errors::storage_error`
- Phase: `6.1`
- Status: `not_started`

## Exported API
### Default class `StorageError`
- Extends `MiddlewareError`.
- Readonly fields:
  - `storageErrorCode: string`
  - `storageErrorMessage: string`
  - `storageRequestID: string`
- Constructor:
  - `new StorageError(statusCode: number, storageErrorCode: string, storageErrorMessage: string, storageRequestID: string, storageAdditionalErrorMessages: { [key: string]: string } = {})`

## Dependencies
- Imports:
  - `../generated/errors/MiddlewareError` — generated middleware base error from Phase `5.13`.
  - `../generated/utils/xml.jsonToXML` — XML serializer from Phase `5.6`.
- Key consumers:
  - `src/blob/errors/StorageErrorFactory.ts` constructs nearly every blob-service error through this type.
  - `src/blob/errors/NotImplementedError.ts` and `src/blob/errors/StrictModelNotSupportedError.ts` subclass it.
  - Blob handlers, middlewares, and authenticators consume the resulting errors indirectly through `StorageErrorFactory`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| class extends `MiddlewareError` | `struct StorageError` plus `impl std::error::Error` | Keep blob-specific fields explicit even if a shared error wrapper exists. |
| `number` HTTP status | `http::StatusCode` or `u16` | Preserve exact status values because they are wire-visible. |
| `{ [key: string]: string }` | `BTreeMap<String, String>` | Stable ordering helps deterministic XML output if tests snapshot the body. |
| XML body string | `String` | Current TS constructor eagerly serializes XML. |

## Function and constructor mapping
- TS constructor:
  - `new StorageError(statusCode, code, message, requestId, extra)`
- Rust recommendation:
  - `pub fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>, request_id: impl Into<String>, extra: BTreeMap<String, String>) -> Self`

## Special handling
- The XML body is built from `{ Error: bodyInJSON }`, where `bodyInJSON.Message` is not the raw message. It is expanded to `${storageErrorMessage}\nRequestId:${storageRequestID}\nTime:${new Date().toISOString()}` at construction time.
- `storageAdditionalErrorMessages` are copied into the XML body as peer elements of `Code` and `Message`; they are **not** surfaced in headers.
- The superclass is called with `storageErrorMessage` in both message-like slots. Do not “improve” that duplication without checking generated middleware expectations.
- The response always sets:
  - `x-ms-error-code: <storageErrorCode>`
  - `x-ms-request-id: <storageRequestID>`
  - content type `application/xml`
- The timestamp is generated when the error is instantiated, not inherited from `context.startTime`.

## Change propagation notes
- Any change to XML element names, header names, or the `Message` expansion format will affect all blob error responses because `StorageErrorFactory` funnels through this constructor.
- If generated `MiddlewareError` changes constructor shape, audit every call site in this file before Aragorn ports the blob error stack.
- If future TS adds non-string extra XML elements, keep the Rust representation close to the TS `string`-only contract until a deliberate team decision says otherwise.
