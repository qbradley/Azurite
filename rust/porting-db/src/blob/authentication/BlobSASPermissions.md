# Porting Record — `src/blob/authentication/BlobSASPermissions.ts`

## File info
- Source path: `src/blob/authentication/BlobSASPermissions.ts`
- Source lines: `13`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/authentication/blob_sas_permissions.rs`
- Crate: `azurite-blob`
- Module: `authentication::blob_sas_permissions`
- Phase: `7.4`
- Status: `not_started`

## Exported API
### Enum `BlobSASPermission`
- `Read = "r"`
- `Add = "a"`
- `Create = "c"`
- `Write = "w"`
- `Delete = "d"`
- `DeleteVersion = "x"`
- `Tag = "t"`
- `Move = "m"`
- `execute = "e"`
- `SetImmutabilityPolicy = "i"`
- `permanentDelete = "y"`

## Dependencies
- Imports: none.
- Key consumers:
  - `OperationBlobSASPermission.ts` concatenates these values into permission requirements.
  - `BlobSASAuthenticator.ts` checks for `Write` in special existing-blob cases.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| string enum | `enum BlobSasPermission` with `as_str()` or `as_char()` | Preserve exact wire characters. |

## Special handling
- Member casing is inconsistent: `execute` and `permanentDelete` are lowercase enum members while the rest are PascalCase. Preserve the inconsistency in the record and avoid “fixing” it without approval.
- This file only defines constants; there is no parse or canonical-order helper here. Later permission logic uses string inclusion directly.

## Change propagation notes
- If TS adds new blob SAS permission characters, update `OperationBlobSASPermission.ts` and `BlobSASAuthenticator.ts` together because they reason about these values as raw strings.
