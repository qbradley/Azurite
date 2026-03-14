# Porting Record — `src/blob/lease/BlobReadLeaseValidator.ts`

## File info
- Source path: `src/blob/lease/BlobReadLeaseValidator.ts`
- Source lines: `36`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/blob_read_lease_validator.rs`
- Crate: `azurite-blob`
- Module: `lease::blob_read_lease_validator`
- Phase: `8.13`
- Status: `not_started`

## Exported API
### Default class `BlobReadLeaseValidator`
- Implements `ILeaseValidator`.
- Constructor: `new BlobReadLeaseValidator(leaseAccessConditions?: LeaseAccessConditions)`
- Method: `validate(lease: ILease, context: Context): void`

## Dependencies
- `LeaseAccessConditions` and `LeaseStatusType` from generated models.
- `StorageErrorFactory` for blob read errors.
- `ILeaseValidator` / `ILease` from `./ILeaseState`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `LeaseAccessConditions` | `Option<LeaseAccessConditions>` | Validation only triggers when the caller supplied a non-empty lease ID. |
| `validate(...)` | `fn validate(...) -> Result<(), StorageError>` | The TS implementation is purely synchronous and throws on failure. |

## Special handling
- `BlobReadLeaseValidator.ts:14-34` is permissive: if the request omits `leaseAccessConditions.leaseId` or passes an empty string, the validator does nothing.
- `BlobReadLeaseValidator.ts:21-33` throws `BlobLeaseLost` when the blob is unlocked but the caller supplied a lease ID, and otherwise compares IDs case-insensitively before throwing `BlobLeaseIdMismatchWithBlobOperation`.

## Change propagation notes
- If read operations ever become stricter in TS, update this validator and every blob read API that wires it in.
- Keep the case-insensitive comparison behavior visible; normalizing IDs elsewhere would hide that contract.
