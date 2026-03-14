# Porting Record — `src/blob/authentication/AccountSASAuthenticator.ts`

## File info
- Source path: `src/blob/authentication/AccountSASAuthenticator.ts`
- Source lines: `392`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/authentication/account_sas_authenticator.rs`
- Crate: `azurite-blob`
- Module: `authentication::account_sas_authenticator`
- Phase: `7.11`
- Status: `ported`

## Exported API
### Default class `AccountSASAuthenticator`
- Implements `IAuthenticator`.
- Constructor:
  - `new AccountSASAuthenticator(accountDataStore: IAccountDataStore, blobMetadataStore: IBlobMetadataStore, logger: ILogger)`
- Public method:
  - `validate(req: IRequest, context: Context): Promise<boolean | undefined>`

### Internal helpers that shape the Rust port
- `getAccountSASSignatureValuesFromRequest(req)`
- `validateTime(expiry, start?)`
- `validateIPRange()`
- `validateProtocol(sasProtocol = "https,http", requestProtocol)`
- `decodeIfExist(value?)`
- `blobExist(account, container, blob)`

## Dependencies
- Imports:
  - `../../common/IAccountDataStore`, `../../common/ILogger`
  - `../errors/StorageErrorFactory` — Phase `6.2`
  - `../generated/artifacts/models.BlobType` and `../generated/artifacts/operation`
  - `../generated/Context`, `../generated/IRequest`
  - `../persistence/IBlobMetadataStore`
  - `../../common/authentication/AccountSASPermissions.AccountSASPermission` — Phase `3.2`
  - `../../common/authentication/IAccountSASSignatureValues` — Phase `3.5` signature generator and interface
  - `./OperationAccountSASPermission` — Phase `7.8`
  - `../errors/StrictModelNotSupportedError` — Phase `6.4`
  - blob constants including `AUTHENTICATION_BEARERTOKEN_REQUIRED`
- Key consumers:
  - Blob authentication middleware/request-listener chain.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| authenticator class | struct with injected account store, blob metadata store, and logger | Keep injected dependencies explicit. |
| `Promise<boolean | undefined>` | `Result<Option<bool>, StorageError>` | This authenticator mostly returns `Some(true)` / `Some(false)`; it rarely uses `None`. |
| decoded query strings | `Option<String>` | TS decodes with `decodeURIComponent()` for every extracted field. |

## Validation flow
1. Read `account`, `container`, and `blob` directly from `context.context` (the file avoids constructing `BlobStorageContext` because it "wants to move this class into common").
2. Resolve the account; unknown accounts throw `ResourceNotFound`.
3. Decode `sig` from the query string and extract account-SAS values from `sv`, `ss`, `srt`, `spr`, `st`, `se`, `sip`, `sp`, `sig`, and `ses`.
4. If required values are missing, return `false` rather than `undefined`.
5. In non-loose mode, reject `ses` with `StrictModelNotSupportedError("SAS Encryption Scope 'ses'", ...)`.
6. Generate the signature with `key1`; if present, also try `key2`. If neither matches, return `false`.
7. Enforce time, IP-range, and protocol checks.
8. Reject `Operation.Service_GetUserDelegationKey` with `AuthenticationFailed(AUTHENTICATION_BEARERTOKEN_REQUIRED)`.
9. Lookup the current operation in `OPERATION_ACCOUNT_SAS_PERMISSIONS` and enforce service/resource-type/permission checks.
10. For create/copy operations, if the destination blob already exists, require `Write` permission explicitly.

## Special handling
- `getAccountSASSignatureValuesFromRequest()` requires all of `sv`, `se`, `sp`, `ss`, `srt`, and `sig`. There is no “identifier-only” account-SAS mode here.
- Missing or malformed account-SAS query data returns `false`, not `undefined`, so this authenticator behaves like a failed match once it is reached.
- `validateIPRange()` is a stub that always returns `true` with a TODO comment saying the emulator does not validate IP addresses.
- `validateProtocol()` returns `true` whenever the SAS protocol string contains a comma. It does not actually confirm that the request protocol is one of the listed values.
- `decodeIfExist()` blindly applies `decodeURIComponent()` to every extracted field.
- `blobExist()` treats uncommitted block blobs as nonexistent for the special write-only rule.
- `generateAccountSASSignature()` is run against both keys before protocol/time/permission enforcement; signature validity gates the rest of the checks.
- Like `BlobSharedKeyAuthenticator`, this file throws if `context.operation` is undefined and uses `AUTHENTICATION_BEARERTOKEN_REQUIRED` for `Service_GetUserDelegationKey`.

## Change propagation notes
- Any Phase `3.x` change to account-SAS serialization or sentinel handling must be audited here and in `OperationAccountSASPermission.ts`.
- If TS later implements real IP-range enforcement, keep the stub-to-real transition explicit in the record because it changes observable auth behavior.
- Preserve the post-signature special-case blob existence check; it is how TS compensates for permissive ANY-character permission matching in the operation table.
