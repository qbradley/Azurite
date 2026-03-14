# Porting Record — `src/blob/lease/BlobWriteLeaseValidator.ts`

## File info
- Source path: `src/blob/lease/BlobWriteLeaseValidator.ts`
- Source lines: `40`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/blob_write_lease_validator.rs`
- Crate: `azurite-blob`
- Module: `lease::blob_write_lease_validator`
- Phase: `8.14`
- Status: `ported`

## Exported API
### Default class `BlobWriteLeaseValidator`
- Implements `ILeaseValidator`.
- Constructor: `new BlobWriteLeaseValidator(leaseAccessConditions?: LeaseAccessConditions)`
- Method: `validate(lease: ILease, context: Context): void`

## Dependencies
- `LeaseAccessConditions` and `LeaseStatusType` from generated models.
- `StorageErrorFactory` for blob write errors.
- `ILeaseValidator` / `ILease` from `./ILeaseState`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `LeaseAccessConditions` | `Option<LeaseAccessConditions>` | Write operations treat missing or empty lease IDs as errors when the lease is locked. |
| `validate(...)` | `fn validate(...) -> Result<(), StorageError>` | Keep the exact branch-specific error selection. |

## Special handling
- `BlobWriteLeaseValidator.ts:15-31` requires a non-empty lease ID whenever the blob lease status is `Locked`, then compares IDs case-insensitively.
- `BlobWriteLeaseValidator.ts:32-38` treats supplying a lease ID against an unlocked blob as `BlobLeaseLost`, so callers cannot opportunistically send stale IDs on writes.

## Change propagation notes
- Any expansion of blob write operations should reuse this validator (or its Rust equivalent) rather than reimplementing lease checks inline.
- If TS changes how empty strings are interpreted, update this file and `BlobReadLeaseValidator` together.
