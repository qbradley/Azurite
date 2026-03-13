# Porting Record — `src/blob/authentication/OperationAccountSASPermission.ts`

## File info
- Source path: `src/blob/authentication/OperationAccountSASPermission.ts`
- Source lines: `660`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/authentication/operation_account_sas_permission.rs`
- Crate: `azurite-blob`
- Module: `authentication::operation_account_sas_permission`
- Phase: `7.8`
- Status: `not_started`

## Exported API
### Class `OperationAccountSASPermission`
- Constructor:
  - `new OperationAccountSASPermission(service: string, resourceType: string, permission: string)`
- Methods:
  - `validate(services, resourceTypes, permissions): boolean`
  - `validateServices(services): boolean`
  - `validateResourceTypes(resourceTypes): boolean`
  - `validatePermissions(permissions): boolean`

### Default export `OPERATION_ACCOUNT_SAS_PERMISSIONS`
- `Map<Operation, OperationAccountSASPermission>` populated by a long flat sequence of `.set(...)` calls.
- Covers blob-service account-SAS requirements for service-, container-, and object-level operations.

## Dependencies
- Imports:
  - External `@azure/storage-blob.AccountSASPermissions`, `AccountSASResourceTypes`, `AccountSASServices` for method parameter types.
  - Local `../../common/authentication/AccountSASPermissions.AccountSASPermission` — Phase `3.2` enum constants used in the table.
  - Local `../../common/authentication/AccountSASResourceTypes.AccountSASResourceType` — Phase `3.4` table values.
  - Local `../../common/authentication/AccountSASServices.AccountSASService` — Phase `3.3` table values.
  - Generated `../generated/artifacts/operation` — Phase `5.11` operation enum.
- Key consumers:
  - `AccountSASAuthenticator.ts` looks up the current operation here before enforcing service/resource/permission checks.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| class with three strings | simple struct with `service`, `resource_type`, `permission` strings | Keep raw-string matching instead of eagerly normalizing into bitflags. |
| `Map<Operation, ...>` | `Lazy<HashMap<Operation, OperationAccountSasPermission>>` | Preserve the explicit one-entry-per-operation table. |
| external helper object or raw string inputs | overload via enums/untagged unions or plain strings | TS relies on `.toString()` against both helper objects and raw strings. |

## Special handling
- Method parameter types come from the external Azure SDK helper classes, but table values come from Azurite's local enums. Preserve that asymmetry so later TS changes remain visible.
- `validateServices()` is a straight `services.toString().includes(this.service)` substring check.
- `validateResourceTypes()` and `validatePermissions()` implement **ANY-character** matching, not ALL-character matching. If `this.permission` is `"wc"`, the request passes when the SAS string contains either `w` or `c`.
- Sentinel handling is special-cased:
  - `AccountSASResourceType.Any` means “resourceTypes string is non-empty”.
  - `AccountSASPermission.Any` means “permissions string is non-empty”.
  - The file comment says these sentinel cases are only for blob batch operations.
- Empty permission strings are meaningful. `Container_SetAccessPolicy` and `Container_GetAccessPolicy` are explicitly mapped with `"" // NOT ALLOWED`, so validation will always fail for those operations.
- The leading `Service_GetAccountInfo` / `*_GetAccountInfoWithHead` mappings concatenate `AccountSASPermission.Read` twice. Keep the duplicate; it is part of the authored source data.
- There are TODO comments marking missing swagger coverage (`Check all required operations`, `Get container metadata is missing in swagger`, `Get blob metadata is missing in swagger`).
- `Operation.Blob_GetProperties` is inserted twice: once normally and once again under the “blob metadata is missing in swagger” comment. The second write overwrites the first with the same value, so behavior stays the same, but the duplication is real source structure.
- Create-style operations (`BlockBlob_Upload`, `PageBlob_Create`, `AppendBlob_Create`, `Blob_StartCopyFromURL`, `Blob_CopyFromURL`) are intentionally mapped to `Write + Create`; `AccountSASAuthenticator` later adds the “if blob exists, Write must be present” special case.

## Change propagation notes
- If the generated `Operation` enum changes order or names, audit this table and `AccountSASAuthenticator` together; the map is keyed directly by those generated members.
- If TS adds new account-SAS permission or resource-type characters, update the local Phase 3 helpers and this table in one pass.
- Do not compress this table into derived logic unless the team explicitly decides to diverge; future TS updates are easiest to propagate when the Rust port keeps one obvious entry per operation.
