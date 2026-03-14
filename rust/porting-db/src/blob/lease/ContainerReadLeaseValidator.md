# Porting Record — `src/blob/lease/ContainerReadLeaseValidator.ts`

## File info
- Source path: `src/blob/lease/ContainerReadLeaseValidator.ts`
- Source lines: `36`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/container_read_lease_validator.rs`
- Crate: `azurite-blob`
- Module: `lease::container_read_lease_validator`
- Phase: `8.17`
- Status: `not_started`

## Exported API
### Default class `ContainerReadLeaseValidator`
- Implements `ILeaseValidator`.
- Constructor: `new ContainerReadLeaseValidator(leaseAccessConditions?: LeaseAccessConditions)`
- Method: `validate(lease: ILease, context: Context): void`

## Dependencies
- `LeaseAccessConditions` and `LeaseStatusType` from generated models.
- `StorageErrorFactory` for container read errors.
- `ILeaseValidator` / `ILease` from `./ILeaseState`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `LeaseAccessConditions` | `Option<LeaseAccessConditions>` | Like blob reads, validation only activates when the caller supplied a non-empty lease ID. |
| `validate(...)` | `fn validate(...) -> Result<(), StorageError>` | Preserve case-insensitive ID comparison and container-specific errors. |

## Special handling
- `ContainerReadLeaseValidator.ts:14-35` is the container twin of `BlobReadLeaseValidator`: empty/missing request lease IDs bypass validation, unlocked leases raise `ContainerLeaseLost`, and locked leases compare IDs case-insensitively before throwing `ContainerLeaseIdMismatchWithContainerOperation`.

## Change propagation notes
- If TS later unifies container/blob read lease checks, keep the error surface distinct unless the upstream change does so too.
- Any new container read entrypoint should route through this validator rather than duplicating the logic.
