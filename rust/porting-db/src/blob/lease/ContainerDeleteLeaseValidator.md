# Porting Record — `src/blob/lease/ContainerDeleteLeaseValidator.ts`

## File info
- Source path: `src/blob/lease/ContainerDeleteLeaseValidator.ts`
- Source lines: `41`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/container_delete_lease_validator.rs`
- Crate: `azurite-blob`
- Module: `lease::container_delete_lease_validator`
- Phase: `8.16`
- Status: `not_started`

## Exported API
### Default class `ContainerDeleteLeaseValidator`
- Implements `ILeaseValidator`.
- Constructor: `new ContainerDeleteLeaseValidator(leaseAccessConditions?: LeaseAccessConditions)`
- Method: `validate(lease: ILease, context: Context): void`

## Dependencies
- `LeaseAccessConditions` and `LeaseStatusType` from generated models.
- `StorageErrorFactory` for container delete errors.
- `ILeaseValidator` / `ILease` from `./ILeaseState`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `LeaseAccessConditions` | `Option<LeaseAccessConditions>` | Delete validation is stricter about `null` vs empty-string handling than the blob validator. |
| `validate(...)` | `fn validate(...) -> Result<(), StorageError>` | Keep the branch-specific container error helpers. |

## Special handling
- `ContainerDeleteLeaseValidator.ts:16-31` mirrors blob write validation for locked leases, but it treats `null` the same as `undefined` for missing lease IDs.
- `ContainerDeleteLeaseValidator.ts:32-38` only throws `ContainerLeaseLost` when the request supplied a non-null, non-empty lease ID against an unlocked container.
- The validator always uses container-specific mismatch/missing/lost errors; do not reuse blob strings here.

## Change propagation notes
- Any TS change to container delete semantics must update both this validator and the delete-container path in `LokiBlobMetadataStore`.
- Keep the explicit `null` checks; they are stricter than the blob validators and therefore compatibility-sensitive.
