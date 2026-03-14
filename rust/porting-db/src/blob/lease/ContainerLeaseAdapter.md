# Porting Record — `src/blob/lease/ContainerLeaseAdapter.ts`

## File info
- Source path: `src/blob/lease/ContainerLeaseAdapter.ts`
- Source lines: `51`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/container_lease_adapter.rs`
- Crate: `azurite-blob`
- Module: `lease::container_lease_adapter`
- Phase: `8.10`
- Status: `not_started`

## Exported API
### Default class `ContainerLeaseAdapter`
- Implements `ILease`.
- Constructor: `new ContainerLeaseAdapter(container: ContainerModel)`
- Utility: `toString(): string`
- Projects container lease fields into the generic lease payload used by the state machine.

## Dependencies
- `../persistence/IBlobMetadataStore.ContainerModel`.
- `../generated/artifacts/models.LeaseStateType`.
- `./ILeaseState`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `ContainerModel` | container metadata document/view struct | Preserve the split between top-level lease timing fields and nested container properties. |
| `implements ILease` | lease payload struct | Container lease operations consume the same generic state machine as blob leases. |
| `toString(): JSON` | `Debug` / `serde_json` helper | Matches the blob adapter debugging style. |

## Special handling
- `ContainerLeaseAdapter.ts:18-29` is stricter than the blob adapter: undefined `leaseState` or `leaseStatus` immediately throws `RangeError`.
- `ContainerLeaseAdapter.ts:31-37` otherwise mirrors the blob projection field-for-field.
- This asymmetry between blob and container adapters is observable and should stay documented.

## Change propagation notes
- If the persistence layer starts normalizing missing container lease fields, re-check whether this adapter should still reject undefined state/status.
- Keep blob/container adapter behavior separate even if the Rust types look similar; the TS callers rely on the different tolerance levels.
