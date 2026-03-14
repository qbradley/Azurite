# Porting Record — `src/blob/lease/ILeaseState.ts`

## File info
- Source path: `src/blob/lease/ILeaseState.ts`
- Source lines: `31`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/i_lease_state.rs`
- Crate: `azurite-blob`
- Module: `lease::i_lease_state`
- Phase: `8.1`
- Status: `not_started`

## Exported API
### Interface `ILease`
- `leaseId?: string`
- `leaseState?: Models.LeaseStateType`
- `leaseStatus?: Models.LeaseStatusType`
- `leaseDurationType?: Models.LeaseDurationType`
- `leaseDurationSeconds?: number`
- `leaseExpireTime?: Date`
- `leaseBreakTime?: Date`

### Interface `ILeaseValidator`
- `validate(lease: ILease, context: Context): void`

### Interface `ILeaseSyncer<T>`
- `sync(lease: ILease): T`

### Interface `ILeaseState`
- `lease: ILease`
- `acquire(duration: number, proposedLeaseId?: string): ILeaseState`
- `break(breakPeriod?: number): ILeaseState`
- `renew(leaseId: string): ILeaseState`
- `change(leaseId: string, proposedLeaseId: string): ILeaseState`
- `release(leaseId: string): ILeaseState`
- `validate(validator: ILeaseValidator): ILeaseState`
- `sync<T>(syncer: ILeaseSyncer<T>): T`

## Dependencies
- `../generated/artifacts/models` for lease enums.
- `../generated/Context` for request-scoped timestamps and context IDs.
- Implemented by all concrete lease state classes and consumed by adapters, validators, syncers, and `LeaseFactory`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `ILease` | `LeaseData` struct | Keep every field optional because TS treats partially-populated leases as meaningful during state reconstruction. |
| `ILeaseState` | `trait` or `enum`-backed dispatch | The interface is synchronous and transition-oriented; methods return the next state, not side-effect futures. |
| `ILeaseValidator` / `ILeaseSyncer<T>` | validator/syncer traits | These are the double-dispatch extension points used by `LeaseStateBase.validate()` and `.sync()`. |

## Special handling
- `ILeaseState.ts:4-11` models lease timing data directly on the state payload; there is no separate scheduler or timer object.
- `ILeaseState.ts:22-30` makes validation and persistence write-back explicit protocol steps (`validate()` then `sync()`), which is why later TS code stays generic over blob vs container leases.
- All operations are synchronous throws/returns; Rust should preserve that shape and only wrap outer storage calls in async.

## Change propagation notes
- If TS adds a new lease field or operation, update this interface, both adapters, all syncers, and `LeaseFactory` together.
- Any shift from synchronous throws to async validation would ripple through `LokiBlobMetadataStore` lease call sites immediately.
