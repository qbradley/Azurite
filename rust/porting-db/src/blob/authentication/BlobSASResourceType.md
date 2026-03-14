# Porting Record — `src/blob/authentication/BlobSASResourceType.ts`

## File info
- Source path: `src/blob/authentication/BlobSASResourceType.ts`
- Source lines: `5`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/authentication/blob_sas_resource_type.rs`
- Crate: `azurite-blob`
- Module: `authentication::blob_sas_resource_type`
- Phase: `7.5`
- Status: `ported`

## Exported API
### Enum `BlobSASResourceType`
- `Container = "c"`
- `Blob = "b"`
- `BlobSnapshot = "bs"`

## Dependencies
- Imports: none.
- Key consumers:
  - `IBlobSASSignatureValues.ts` branches on this enum while building canonical names.
  - `BlobSASAuthenticator.ts` validates incoming `sr` query values against these three cases.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| string enum | `enum BlobSasResourceType` with `as_str()` | Keep the two-character snapshot value `bs` explicit. |

## Special handling
- Snapshot SAS is not fully symmetrical with blob/container SAS. Some signature helpers treat `BlobSnapshot` like `Blob`, while later UDK helpers and permission routing do not.
- Do not collapse `BlobSnapshot` into `Blob` in Rust, even if later validation shares code paths.

## Change propagation notes
- If Azure adds new blob SAS resource kinds, audit both the signature generators and `BlobSASAuthenticator` permission-table routing; they are not isolated concerns.
