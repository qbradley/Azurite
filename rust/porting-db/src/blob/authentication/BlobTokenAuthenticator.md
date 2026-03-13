# Porting Record — `src/blob/authentication/BlobTokenAuthenticator.ts`

## File info
- Source path: `src/blob/authentication/BlobTokenAuthenticator.ts`
- Source lines: `253`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/authentication/blob_token_authenticator.rs`
- Crate: `azurite-blob`
- Module: `authentication::blob_token_authenticator`
- Phase: `7.13`
- Status: `not_started`

## Exported API
### Default class `BlobTokenAuthenticator`
- Implements `IAuthenticator`.
- Constructor:
  - `new BlobTokenAuthenticator(dataStore: IAccountDataStore, oauth: OAuthLevel, logger: ILogger)`
- Public methods:
  - `validate(req: IRequest, context: Context): Promise<boolean | undefined>`
  - `authenticateBasic(token: string, context: Context): Promise<boolean>`

## Dependencies
- Imports:
  - External `jsonwebtoken.decode` — token parsing only.
  - `../../common/IAccountDataStore`, `../../common/ILogger`, `../../common/models.OAuthLevel`
  - `../../common/utils/constants` — `BEARER_TOKEN_PREFIX`, `HTTPS`, `VALID_ISSUE_PREFIXES`
  - `../context/BlobStorageContext` — Phase `6.5`
  - `../errors/StorageErrorFactory` — Phase `6.2`
  - generated `Operation`, `Context`, `IRequest`
  - blob constants `HeaderConstants`, `VALID_BLOB_AUDIENCES`
  - `./IAuthenticator`
- Key consumers:
  - Blob authentication middleware/request-listener chain.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| authenticator class | struct with injected store, oauth mode, and logger | Keep BASIC-vs-other dispatch explicit. |
| JWT payload as loose object | serde JSON map / strongly typed claims struct with optional fields | TS checks a handful of claims manually. |
| `Promise<boolean | undefined>` | `Result<Option<bool>, StorageError>` | `None` means token auth was skipped / not applicable. |

## Validation flow
1. Wrap the context as `BlobStorageContext` and read `account`.
2. Lookup the account **before** checking the auth header; unknown accounts throw `ResourceNotFound`.
3. If the `Authorization` header is missing, return `undefined`.
4. Skip token authentication (`undefined`) for `Container_GetAccessPolicy` and `Container_SetAccessPolicy`.
5. Require the header to start with `BEARER_TOKEN_PREFIX`; otherwise throw `InvalidAuthenticationInfo`.
6. Require HTTPS; over HTTP throw `AuthenticationFailed("Authentication scheme Bearer is not allowed with HTTP.")`.
7. Extract the token with `authHeaderValue.substr(BEARER_TOKEN_PREFIX.length + 1)`.
8. Dispatch by OAuth level. Only `OAuthLevel.BASIC` is implemented; unknown levels log a warning and return `undefined`.
9. In `authenticateBasic()`:
   - decode the JWT with `jsonwebtoken.decode()`,
   - do **not** verify its signature,
   - require `nbf`, `exp`, and `iat`,
   - validate lifetime against `context.startTime`,
   - validate `iss` against the allowed issuer prefixes,
   - validate `aud` against `VALID_BLOB_AUDIENCES`, with optional account-name capture enforcement,
   - return `true` on success.

## Special handling
- Signature verification is intentionally skipped in BASIC mode. The source comment explicitly says “Validate signature, skip in basic check”. Preserve that behavior unless the team decides to harden it.
- Time validation uses `context.startTime!.getTime()` rather than a freshly created `new Date()`. That keeps auth timing aligned with request-start timing.
- The audience regex handling is exact-match-style: regex must match and `m[0] === aud` must hold; if a capture group exists and it does not match the current account, validation fails.
- Missing/invalid JWT structure throws `AuthenticationFailed("Authentication scheme Bearer is not supported.")`.
- Scope validation is intentionally skipped.
- The prefix extraction uses `substr(prefix.length + 1)`, assuming a single separating space after `Bearer`.

## Change propagation notes
- If OAuth levels expand beyond BASIC, preserve the explicit mode dispatch; do not fold future behaviors into one catch-all path.
- If issuer or audience constants change, audit this file and the shared/blob constants together because the matching is string/regex based rather than declarative policy based.
- If the team later decides to verify JWT signatures, record that as an explicit semantic change; it is not a mechanical port of current TS behavior.
