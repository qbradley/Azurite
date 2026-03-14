# Porting Record — `src/blob/authentication/ContainerSASPermissions.ts`

## File info
- Source path: `src/blob/authentication/ContainerSASPermissions.ts`
- Source lines: `10`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/authentication/container_sas_permissions.rs`
- Crate: `azurite-blob`
- Module: `authentication::container_sas_permissions`
- Phase: `7.6`
- Status: `ported`

## Exported API
### Enum `ContainerSASPermission`
- `Read = "r"`
- `Add = "a"`
- `Create = "c"`
- `Write = "w"`
- `Delete = "d"`
- `List = "l"`
- `Filter = "f"`
- `Any = "AnyPermission"` — comment says this is only for blob batch operation.

## Dependencies
- Imports: none.
- Key consumers:
  - `OperationBlobSASPermission.ts` uses `Any` as a sentinel for batch permission validation.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| string enum with sentinel | `enum ContainerSasPermission` | Keep `Any` separate from the one-character wire permissions. |

## Special handling
- `AnyPermission` is not a normal service-SAS wire character. It is a validation sentinel consumed by `OperationBlobSASPermission.validatePermissions()`.
- Combined permission strings in later code are checked with “any matching character” semantics, not “all required characters”. Keep the sentinel separate so that behavior remains explicit.

## Change propagation notes
- If blob batch semantics change, update this record and `OperationBlobSASPermission.ts` together.
