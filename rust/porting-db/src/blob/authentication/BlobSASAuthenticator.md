# Porting Record — `src/blob/authentication/BlobSASAuthenticator.ts`

## File info
- Source path: `src/blob/authentication/BlobSASAuthenticator.ts`
- Source lines: `627`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/authentication/blob_sas_authenticator.rs`
- Crate: `azurite-blob`
- Module: `authentication::blob_sas_authenticator`
- Phase: `7.12`
- Status: `ported`

## Exported API
### Default class `BlobSASAuthenticator`
- Implements `IAuthenticator`.
- Constructor:
  - `new BlobSASAuthenticator(accountDataStore: IAccountDataStore, blobMetadataStore: IBlobMetadataStore, logger: ILogger)`
- Public method:
  - `validate(req: IRequest, context: Context): Promise<boolean | undefined>`

### Internal helpers that shape the Rust port
- `getBlobSASSignatureValuesFromRequest(req, containerName, blobName?, context?)`
- `validateTime(expiry?, start?)`
- `validateIPRange()`
- `validateProtocol(sasProtocol = "https,http", requestProtocol)`
- `decodeIfExist(value?)`
- `getContainerAccessPolicyByIdentifier(account, container, id, context)`
- `blobExist(account, container, blob)`

## Dependencies
- Imports:
  - `../../common/IAccountDataStore`, `../../common/ILogger`
  - `../context/BlobStorageContext` — Phase `6.5`
  - `../errors/StorageErrorFactory` — Phase `6.2`
  - `../errors/StrictModelNotSupportedError` — Phase `6.4`
  - generated `AccessPolicy`, `BlobType`, `Operation`, `Context`, `IRequest`
  - `../persistence/IBlobMetadataStore`
  - blob constants `AUTHENTICATION_BEARERTOKEN_REQUIRED`
  - `../utils/utils.getUserDelegationKeyValue`
  - `./BlobSASPermissions`, `./BlobSASResourceType`, `./IBlobSASSignatureValues`, `./OperationBlobSASPermission`
- Key consumers:
  - Blob authentication middleware/request-listener chain.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| complex authenticator class | struct with injected stores/logger | Keep flow-oriented methods rather than over-abstracting. |
| service SAS vs UDK branching | explicit enum/branch in Rust | The two paths share later validation but differ at signature-generation time. |
| `Promise<boolean | undefined>` | `Result<Option<bool>, StorageError>` | This authenticator genuinely uses all three states. |

## Validation flow
1. Wrap `Context` as `BlobStorageContext`; require `account`, but treat missing `container` as `undefined` / not applicable.
2. Lookup the account; missing accounts throw `ResourceNotFound`.
3. Decode `sig`; if missing, return `undefined`.
4. Decode `sr`; if it is not `c`, `b`, or `bs`, return `undefined`.
5. Extract blob-SAS values from the request. Missing/invalid values return `undefined`.
6. In non-loose mode, reject `ses` with `StrictModelNotSupportedError("SAS Encryption Scope 'ses'", ...)`.
7. If any user-delegation-key field is present, enter the UDK path:
   - require all six signed key fields and `signedService === "b"`,
   - reject saved-policy (`si`) usage,
   - validate signed UDK time window,
   - derive the key with `getUserDelegationKeyValue(...)`,
   - compute the UDK signature with `generateBlobSASSignatureWithUDK(...)`,
   - return `false` immediately if the signature mismatches.
8. Otherwise compute standard shared-key SAS signatures with `key1`, then `key2` if present.
9. If `identifier` is present, fetch the container ACL and override `startTime`, `expiryTime`, and `permissions` from the saved access policy.
10. Enforce time, IP-range, and protocol checks.
11. Reject `Service_GetUserDelegationKey` with bearer-token-required auth failure.
12. Choose the permission table:
   - `resource === Blob` → `OPERATION_BLOB_SAS_BLOB_PERMISSIONS`
   - otherwise (`Container` **and** `BlobSnapshot`) → `OPERATION_BLOB_SAS_CONTAINER_PERMISSIONS`
13. Enforce permission validation and the existing-blob Write special case for create/copy operations.
14. Return `true` on success.

## Special handling
- `getBlobSASSignatureValuesFromRequest()` decodes `sv`, `spr`, `st`, `se`, `sp`, `sip`, `si`, `sr`, `snapshot`, `ses`, `skoid`, `sktid`, `skt`, `ske`, `skv`, and `sks`, but it reads `rscc`, `rscd`, `rsce`, `rscl`, and `rsct` **without** `decodeURIComponent()`. Preserve that asymmetry.
- Unlike the generator module, request extraction requires `permissions` **and** `expiryTime` when `identifier` is absent (`if (!identifier && (!permissions || !expiryTime))`).
- `getBlobSASSignatureValuesFromRequest()` does **not** extract the future delegated-user fields that exist on `IBlobSASSignatureValues` (`delegatedUserObjectId`, `delegatedUserTenantId`). The generator knows about them for `2025-07-05`, but the authenticator does not currently parse them from requests.
- `validateIPRange()` is still a stub that always returns `true`.
- `validateProtocol()` has the same permissive comma-handling as `AccountSASAuthenticator`: any comma in the SAS protocol string passes.
- Saved access policies only override `startTime`, `expiryTime`, and `permissions`. Protocol, IP range, response-header overrides, encryption scope, and resource fields are left unchanged.
- `BlobSnapshot` is not treated like `Blob` when selecting the permission map. Snapshot SAS requests are validated against the **container** permission table.
- The special existing-blob rule requires `BlobSASPermission.Write` when the destination blob already exists for upload/create/copy operations.
- `blobExist()` again treats uncommitted block blobs as nonexistent.
- There is a TODO at the end for enforced response headers defined in blob service SAS; the authenticator validates signatures over those fields but does not apply them to responses yet.

## Change propagation notes
- Any Phase `7.3` change to blob-SAS string-to-sign fields must be checked here and in `getBlobSASSignatureValuesFromRequest()` together; the parser and signer are not fully symmetric today.
- If TS later adds a dedicated snapshot permission table or starts parsing delegated-user fields, document that explicitly before Aragorn ports the auth flow.
- Keep the UDK branch separate in Rust. It is not just a key-substitution detail; it also rejects saved policies and validates a different timestamp envelope.
