# Porting Record — `src/blob/authentication/BlobSharedKeyAuthenticator.ts`

## File info
- Source path: `src/blob/authentication/BlobSharedKeyAuthenticator.ts`
- Source lines: `344`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/authentication/blob_shared_key_authenticator.rs`
- Crate: `azurite-blob`
- Module: `authentication::blob_shared_key_authenticator`
- Phase: `7.10`
- Status: `not_started`

## Exported API
### Default class `BlobSharedKeyAuthenticator`
- Implements `IAuthenticator`.
- Constructor:
  - `new BlobSharedKeyAuthenticator(dataStore: IAccountDataStore, logger: ILogger)`
- Public method:
  - `validate(req: IRequest, context: Context): Promise<boolean | undefined>`

### Internal helpers that shape the Rust port
- `getHeaderValueToSign(request, headerName): string`
- `getCanonicalizedHeadersString(request): string`
- `getCanonicalizedResourceString(request, account, authenticationPath?): string`

## Dependencies
- Imports:
  - `../../common/IAccountDataStore` — account key lookup.
  - `../../common/ILogger` — logging side effects throughout validation.
  - `../../common/utils/utils.computeHMACSHA256` and `getURLQueries` — shared HMAC and URL-query helpers.
  - `../context/BlobStorageContext` — Phase `6.5` wrapper around generated context.
  - `../errors/StorageErrorFactory` — Phase `6.2` auth/resource errors.
  - `../generated/artifacts/operation` — Phase `5.11` operation enum.
  - `../generated/Context`, `../generated/IRequest` — generated request/context types.
  - `../utils/constants` — blob constants including `HeaderConstants` and `AUTHENTICATION_BEARERTOKEN_REQUIRED`.
  - `./IAuthenticator` — Phase `7.1`.
- Key consumers:
  - Authentication middleware/request-listener code wires this into the blob authenticator chain.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| authenticator class with logger/data store | struct with injected trait objects / concrete services | Keep constructor-injected dependencies explicit. |
| `Promise<boolean | undefined>` | `Result<Option<bool>, StorageError>` | `None` means this auth pattern did not apply. |
| canonicalized headers/resource strings | dedicated helper fns returning `String` | Preserve exact string assembly; this is the signature contract. |

## Validation flow
1. Wrap incoming `Context` as `BlobStorageContext` and read `account`.
2. If the `Authorization` header is missing, return `undefined`.
3. If the header does not start with `"SharedKey"`, return `undefined`.
4. Lookup the account. Missing account throws `StorageErrorFactory.ResourceNotFound(...)`.
5. Reject `Operation.Service_GetUserDelegationKey` with `AuthenticationFailed(AUTHENTICATION_BEARERTOKEN_REQUIRED)` because that path requires OAuth.
6. Build the primary string-to-sign from method, selected standard headers, canonicalized `x-ms-*` headers, and canonicalized resource.
7. Compare against `key1`; if present, compare against `key2` as fallback.
8. If the request is secondary and `authenticationPath?.indexOf(account) === 1`, build a second string-to-sign with `-secondary` inserted after the account name inside `authenticationPath`, then retry key1/key2.
9. Return `false` if all comparisons fail.

## Special handling
- `getHeaderValueToSign()` returns `""` for missing headers and also for `Content-Length: 0`. Preserve that special case exactly.
- The canonical string uses `req.getMethod().toUpperCase()` and then joins the selected standard headers with `"\n"`, appending another `"\n"` before canonicalized headers/resources.
- `getCanonicalizedHeadersString()`:
  - collects all request headers,
  - turns array values into comma-joined strings,
  - filters to names starting with `HeaderConstants.PREFIX_FOR_STORAGE`,
  - sorts case-insensitively,
  - emits `${name.toLowerCase().trimRight()}:${value.trimLeft()}\n` for each header.
- The comments describe replacing linear whitespace with a single space, but the implementation does **not** do that normalization. Preserve the implementation, not the prose comment.
- `getCanonicalizedResourceString()` lowercases query keys, sorts them, and appends `\n${key}:${decodeURIComponent(value.replace(/\+/g, '%20'))}` for each query value.
- For secondary endpoints, the authenticator only performs the alternate `-secondary` signature path when `context.context.isSecondary` is true **and** `authenticationPath` begins with `/{account}` (detected via `indexOf(account) === 1`).
- Missing/unknown accounts are treated as resource-not-found, not authentication-failed.
- The `startsWith("SharedKey")` check is prefix-based; it does not insist on a following space.

## Change propagation notes
- Any change to canonicalized header ordering, query decoding, or `Content-Length: 0` handling will break Shared Key signature parity.
- If later blob constants change header names or the `x-ms-` prefix constant, revisit both helper methods together.
- Keep the secondary-endpoint branch explicit in Rust; it is a compatibility path for Track2 SDK IP-style URIs, not a generic aliasing rule.
