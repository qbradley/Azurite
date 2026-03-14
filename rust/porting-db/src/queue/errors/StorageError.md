# Porting Record — `src/queue/errors/StorageError.ts`

## File info
- Source path: `src/queue/errors/StorageError.ts`
- Source lines: `67`
- Source type: `handwritten error class`
- Rust target (per `PORTING-ORDER.md`): `azurite-queue/src/errors/storage_error.rs`
- Crate: `azurite-queue`
- Module: `errors::storage_error`
- Phase: `14.2`
- Status: `not_started`

## Exported API
### Default class `StorageError`
- Extends `MiddlewareError` (Phase 5 generated errors)
- Constructor: `new StorageError(statusCode: number, storageErrorCode: string, storageErrorMessage: string, storageRequestID: string, storageAdditionalErrorMessages?: { [key: string]: string })`
- Public properties (readonly):
  - `storageErrorCode: string`
  - `storageErrorMessage: string`
  - `storageRequestID: string`
  - `name: "StorageError"` (set in constructor)
- Inherits from MiddlewareError: statusCode, statusMessage, headers, body, contentType

## Dependencies
- `MiddlewareError` (Phase 5 generated): Base error class with HTTP response fields
- `jsonToXML()` utility (Phase 5 generated utils): Converts JSON error object to XML body
- `QUEUE_API_VERSION` constant: Included in error response headers

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `extends MiddlewareError` | Struct composition or inheritance | StorageError wraps/extends middleware error |
| `{ [key: string]: string }` | `HashMap<String, String>` | Additional error messages object |
| `jsonToXML()` | Custom function or crate | Convert error object to Azure-standard XML |
| `new Date().toISOString()` | `chrono::Utc::now().to_rfc3339()` | Timestamp in error message |

## Special handling
1. **XML error body construction** (`StorageError.ts:35-47`)
   - Builds JSON object: `{ Code, Message, ...additionalMessages }`
   - Converts to XML via `jsonToXML()` utility
   - XML has structure: `<Error><Code>...</Code><Message>...\nRequestId:...\nTime:...</Message>...</Error>`
   - Message includes requestID and ISO timestamp — must be exact format

2. **Timestamp in message** (`StorageError.ts:37`)
   - ISO 8601 format: `new Date().toISOString()`
   - Included in error message text (human-readable)
   - Not a separate field — timestamp is part of message string

3. **Additional error messages** (`StorageError.ts:40-45`)
   - Optional map of key-value error details
   - Each key becomes a sibling element in XML body
   - Used for supplementary error context (e.g., `{ AuthenticationErrorDetail: "..." }`)
   - Converted directly to XML by `jsonToXML()`

4. **HTTP headers** (`StorageError.ts:53-57`)
   - Custom headers set in parent MiddlewareError constructor:
     - `x-ms-error-code`: Maps storageErrorCode
     - `x-ms-request-id`: Request ID for tracing
     - `x-ms-version`: Queue API version constant
   - Must be exact header names (case-sensitive in HTTP/2)

5. **Content-Type** (`StorageError.ts:59`)
   - Always `application/xml` (not JSON)
   - Fixed in all StorageError instances

6. **Constructor delegation** (`StorageError.ts:49-60`)
   - Builds JSON → XML, then passes to parent constructor
   - Parent constructor sets HTTP response details (status, headers, body)
   - StorageError stores queue-specific fields after super() call

## Change propagation notes
- If `MiddlewareError` signature changes, update constructor call
- If XML serialization changes, audit all error responses (must match Azure SDK expectations)
- Error codes and messages are part of public API — do not normalize/change names
- The message format with `\nRequestId:\nTime:` embedded newlines must be preserved exactly
- If new error types are added, use StorageErrorFactory (Phase 14.3) to centralize error creation

