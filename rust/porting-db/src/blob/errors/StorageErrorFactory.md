# Porting Record — `src/blob/errors/StorageErrorFactory.ts`

## File info
- Source path: `src/blob/errors/StorageErrorFactory.ts`
- Source lines: `854`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/errors/storage_error_factory.rs`
- Crate: `azurite-blob`
- Module: `errors::storage_error_factory`
- Phase: `6.2`
- Status: `ported`

## Exported API
### Default class `StorageErrorFactory`
- Pure static factory class over `StorageError`.
- Internal constant: `DefaultID = "DefaultBlobRequestID"`.
- Exposes 74 static constructors returning `StorageError`.

### Method families
- **Existence / conflict**: `getContainerNotFound`, `getContainerAlreadyExists`, `getBlobAlreadyExists`, `getBlobNotFound`, `ResourceNotFound`.
- **Request validation / query / headers**: `getInvalidQueryParameterValue`, `getOutOfRangeInput`, `getInvalidOperation`, `getInvalidAuthenticationInfo`, `getMd5Mismatch`, `getInvalidHeaderValue`, `getInvalidAPIVersion`, `getInvalidResourceName`, `getOutOfRangeName`, `getInvalidXmlDocument`, `getInvalidMetadata`, `getEmptyTagName`, `getDuplicateTagNames`, `getTagsTooLarge`, `getInvalidTag`.
- **Range / content**: `getRequestEntityTooLarge`, `getBlockCountExceedsLimit`, `getInvalidBlockList`, `getInvalidPageRange`, `getInvalidPageRange2`, `getInvalidBlobOrBlock`.
- **Lease / condition state**: container lease errors, blob lease errors, conditional header failures, append/page/blob size condition failures.
- **Authentication / authorization**: `getAuthorizationFailure`, `getAuthenticationFailed`, `getAuthorizationSourceIPMismatch`, `getAuthorizationProtocolMismatch`, `getAuthorizationPermissionMismatch`, `getAuthorizationServiceMismatch`, `getAuthorizationResourceTypeMismatch`.
- **Copy / tier / snapshot**: snapshot errors, copy ID/state errors, sync-copy status, access tier and archive errors.
- **CORS**: `getInvalidCorsHeaderValue`, `corsPreflightFailure`.

## Dependencies
- Imports:
  - `./StorageError` — Phase `6.1` base blob error type.
- Key consumers:
  - Blob authenticators use the auth- and SAS-related factories.
  - Lease, copy, metadata, and handler code across `src/blob/` throw these helpers directly.
  - `StrictModelNotSupportedError.ts` and `NotImplementedError.ts` are separate thin subclasses rather than using this factory.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| static factory class | module of `pub fn` helpers or `impl StorageErrorFactory` with associated fns | No instance state exists. |
| `DefaultID` string constant | `pub const DEFAULT_ID: &str` | Keep the literal because some call sites rely on implicit default IDs. |
| optional extra XML element map | `BTreeMap<String, String>` | Used by query/header/auth/copy helpers. |
| ad-hoc status code integers | explicit `StatusCode` constants | Preserve exact wire mapping, including 304 and 416 edge cases. |

## Recommended Rust translation
- Keep this as a 1:1 helper module rather than collapsing into a single enum too early; the TS porting value is in the named factory methods and their exact messages.
- Group functions by theme in Rust source for readability, but do not rename the externally referenced helper names in the porting database.

## Special handling
- `getInvalidQueryParameterValue()` and `getOutOfRangeInput()` build `additionalMessages` conditionally, adding `QueryParameterName`, `QueryParameterValue`, and `Reason` only when provided.
- `getMd5Mismatch()` adds `UserSpecifiedMd5` and `ServerCalculatedMd5` XML elements.
- `getInvalidPageRange2()` is one of the few helpers that mutates the constructed error after creation by injecting `Content-Range` into `returnValue.headers!` when a header value is supplied.
- `getAuthenticationFailed()` adds `AuthenticationErrorDetail` to the XML body while reusing the same top-level message text as `getAuthorizationFailure()`.
- `getInvalidAPIVersion()` emits a long Azurite-specific upgrade / configuration hint and uses error code `InvalidHeaderValue`, not a dedicated API-version code.
- `getCannotVerifyCopySource()` is the only helper that accepts a caller-provided `statusCode` and message while fixing the storage error code to `CannotVerifyCopySource`.
- `getBlobArchived()` and `getInvalidHeaderValue()` defensively replace `undefined` extra-message maps with `{}`.
- `getNotModified()` returns HTTP `304` but still uses storage code `ConditionNotMet` and the standard conditional-header message.
- `getInvalidTag()` is quirky: it returns error code `DuplicateTagNames` even though the message is about invalid tag characters. Treat that mismatch as source truth unless the team approves divergence.
- `getAuthorizationSourceIPMismatch()` keeps the literal placeholder `{SourceIP}` inside the message; TS does not substitute the actual client IP.
- Some methods intentionally or accidentally reuse generic header-format text (`getInvalidLeaseDuration`, `getInvalidLeaseBreakPeriod`, `getInvalidId`).
- Snapshot handling has three related helpers with different status/message pairs:
  - `getBlobSnapshotsPresent()` → 400, “blob is snapshot” comment says the server match still needs checking.
  - `getBlobSnapshotsPresent_hassnapshot()` → 409, “blob has snapshots”.
  - `getSnapshotsPresent()` → 409, “blob has snapshots”.

## Fidelity risks and edge cases
- Default context IDs are inconsistent: many methods default to `DefaultID`, but some newer helpers default to `""`. Preserve those defaults rather than normalizing them.
- Several permission/state helpers return concatenated meanings that later code depends on semantically, e.g. `Authorization*Mismatch`, `Lease*`, `BlobArchived`, `BlobIsSealed`.
- The file is effectively part of the blob protocol contract; changing error strings or status codes will break parity tests faster than most other handwritten modules.

## Change propagation notes
- Any future TS addition here should become a distinct Rust helper, not a branch inside a generic constructor, so change propagation stays mechanical.
- If `StorageError` constructor shape changes, every factory method needs audit because this file is just a flat list of direct constructor calls.
- If Azurite changes even a single error string, cargo-side tests should snapshot both XML body and headers for the affected operation family before Aragorn rewrites adjacent helpers.
