# Porting Record — `src/queue/errors/StorageErrorFactory.ts`

## File info
- Source path: `src/queue/errors/StorageErrorFactory.ts`
- Source lines: `340`
- Source type: `handwritten static factory class`
- Rust target (per `PORTING-ORDER.md`): `azurite-queue/src/errors/storage_error_factory.rs`
- Crate: `azurite-queue`
- Module: `errors::storage_error_factory`
- Phase: `14.3`
- Status: `ported`

## Exported API
### Default class `StorageErrorFactory`
- All static methods (no instance constructor)
- 27 static factory methods, each returning `StorageError`:
  - **Generic:** `notImplement()`, `InternalError()`, `ResourceNotFound()`, `getRequestBodyTooLarge()`
  - **Authentication/Authorization:** `getAuthenticationFailed()`, `getInvalidAuthenticationInfo()`, `getAuthorizationFailure()`, `getAuthorizationSourceIPMismatch()`, `getAuthorizationProtocolMismatch()`, `getAuthorizationPermissionMismatch()`, `getAuthorizationServiceMismatch()`, `getAuthorizationResourceTypeMismatch()`
  - **Header/Query:** `getInvalidHeaderValue()`, `getInvalidAPIVersion()`, `getInvalidCorsHeaderValue()`, `getInvalidUri()`, `getInvalidOperation()`, `getInvalidXmlDocument()`, `getInvalidQueryParameterValue()`, `getOutOfRangeQueryParameterValue()`
  - **Queue-Specific:** `getQueueAlreadyExists()`, `getQueueNotFound()`, `getMessageNotFound()`, `getPopReceiptMismatch()`, `getMessageTooLarge()`, `getInvalidResourceName()`, `getOutOfRangeName()`, `corsPreflightFailure()`

## Dependencies
- `StorageError` (Phase 14.2): Return type for all factory methods

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `public static method()` | `pub fn` in module or `impl StorageErrorFactory` | Static methods become module functions or associated functions |
| `contextID: string = defaultID` | Parameter with default value | Rust default parameters via builder or separate function variants |
| `additionalMessages?: { [key: string]: string }` | `Option<HashMap<String, String>>` | Optional error details map |
| `defaultID: string = "DefaultID"` | Module-level constant | Used when contextID not provided |
| Return type `StorageError` | Same | Direct return of error instance |

## Special handling
1. **Constant fallback** (`StorageErrorFactory.ts:3`)
   - `defaultID = "DefaultID"` used when contextID parameter omitted
   - Must preserve exact string for compatibility (may be visible in test assertions)

2. **Method naming variance**
   - Some methods use `get` prefix: `getAuthenticationFailed()`, `getInvalidHeaderValue()`
   - Others use no prefix: `notImplement()`, `ResourceNotFound()`, `InternalError()`, `corsPreflightFailure()`
   - Some use verb: `notImplement()` vs noun: `ResourceNotFound()`
   - Pattern is inconsistent and must be preserved exactly

3. **Error code → HTTP status mapping**
   - Each factory method hardcodes specific HTTP status code (400, 403, 404, 500)
   - Error codes are queue-service-specific (not shared with blob)
   - Map must be exactly preserved (e.g., `AuthenticationFailed` → 403, `InvalidHeaderValue` → 400)

4. **Additional messages pattern** (`StorageErrorFactory.ts:34-36`, `208-220`)
   - Optional object parameter for supplementary error details
   - If undefined, factory initializes empty object before passing to StorageError
   - Pattern repeated across multiple methods
   - Some methods accept additionalMessages, others don't

5. **Context-dependent error messages** (`StorageErrorFactory.ts:49-55`)
   - `getInvalidAPIVersion()` includes apiVersion parameter in message string
   - Message suggests user action (upgrade Azurite or use --skipApiVersionCheck)
   - Dynamic message construction from parameter

6. **Queue-specific errors**
   - `getQueueAlreadyExists()` (409 Conflict)
   - `getQueueNotFound()` (404 Not Found)
   - `getMessageNotFound()` (404 Not Found)
   - `getPopReceiptMismatch()` (409 Conflict) — queue message coordination error
   - `getMessageTooLarge()` (413 Payload Too Large) — queue max message size exceeded

## Change propagation notes
- If new queue errors need to be added, follow factory pattern (static method returning StorageError)
- Keep method names consistent with blob-layer equivalents where overlap exists
- Error messages are part of public API — preserve exact wording including punctuation
- Default contextID ("DefaultID") must remain stable for backward compatibility
- HTTP status codes are Azure Storage standard — do not change without understanding impact on clients

