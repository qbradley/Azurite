# Porting Record — `src/blob/authentication/OperationBlobSASPermission.ts`

## File info
- Source path: `src/blob/authentication/OperationBlobSASPermission.ts`
- Source lines: `548`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/authentication/operation_blob_sas_permission.rs`
- Crate: `azurite-blob`
- Module: `authentication::operation_blob_sas_permission`
- Phase: `7.9`
- Status: `not_started`

## Exported API
### Class `OperationBlobSASPermission`
- Constructor: `new OperationBlobSASPermission(permission: string = "")`
- Methods:
  - `validate(permissions: string): boolean`
  - `validatePermissions(permissions: string): boolean`

### Exported permission tables
- `OPERATION_BLOB_SAS_BLOB_PERMISSIONS: Map<Operation, OperationBlobSASPermission>`
- `OPERATION_BLOB_SAS_CONTAINER_PERMISSIONS: Map<Operation, OperationBlobSASPermission>`

## Dependencies
- Imports:
  - Generated `../generated/artifacts/operation` — Phase `5.11`.
  - `./BlobSASPermissions` — Phase `7.4` blob permission enum.
  - `./ContainerSASPermissions` — Phase `7.6` container permission enum and `Any` sentinel.
- Key consumers:
  - `BlobSASAuthenticator.ts` chooses one of the two maps based on signed resource type.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| class with one string | simple wrapper struct or direct helper fn over `String` | Behavior lives in raw-string matching. |
| two populated `Map`s | two `Lazy<HashMap<Operation, OperationBlobSasPermission>>` statics | Keep separate tables rather than merging blob and container scope. |

## Special handling
- `validatePermissions()` treats `ContainerSASPermission.Any` specially: any non-empty permission string passes. This is only used for `Operation.Container_SubmitBatch` in the container-level table.
- Otherwise validation is **ANY-character** matching, not ALL-character matching. A requirement like `Write + Create` passes when either `w` or `c` is present.
- Empty permission strings are not neutral. Because the validation loop runs over `this.permission`, `new OperationBlobSASPermission()` always returns `false` from `validatePermissions()` and therefore models “not allowed” / “not supported for this resource scope”. Do not reinterpret empty strings as “no permission required”.
- The file intentionally duplicates many operations across the blob-level and container-level maps so that `BlobSASAuthenticator` can choose by resource scope.
- TODO comments mark uncertain assignments for `Blob_Undelete`, `Blob_SetTier`, and several create/copy cases where nonexistent-vs-existing destination semantics matter.
- Several create/copy entries are authored as `Write + Create`; later `BlobSASAuthenticator` adds an extra runtime check requiring `Write` when the destination blob already exists.
- There is **no dedicated snapshot permission table**. Later `BlobSASAuthenticator` routes `BlobSnapshot` requests through the container-level table because it only special-cases `resource === Blob`.

## Change propagation notes
- Any change to permission characters in Phase `7.4` / `7.6` must be audited here and in `BlobSASAuthenticator`.
- Keep the two-table structure close to TS; future additions are easier to port if every operation remains a direct table entry rather than a generated rule.
