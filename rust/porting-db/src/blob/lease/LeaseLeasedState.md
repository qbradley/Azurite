# Porting Record — `src/blob/lease/LeaseLeasedState.ts`

## File info
- Source path: `src/blob/lease/LeaseLeasedState.ts`
- Source lines: `295`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/lease_leased_state.rs`
- Crate: `azurite-blob`
- Module: `lease::lease_leased_state`
- Phase: `8.4`
- Status: `not_started`

## Exported API
### Default class `LeaseLeasedState`
- Extends `LeaseStateBase`.
- Constructor: `new LeaseLeasedState(lease: ILease, context: Context)`
- Transition methods:
  - `acquire(duration, proposedLeaseId?) -> LeaseLeasedState`
  - `break(breakPeriod?) -> LeaseBrokenState | LeaseBreakingState`
  - `renew(leaseId) -> LeaseLeasedState | this`
  - `change(leaseId, proposedLeaseId) -> LeaseLeasedState`
  - `release(leaseId) -> LeaseAvailableState`

## Dependencies
- `uuid` for auto-generated IDs during re-acquire paths.
- `../../common/utils/utils.minDate` for fixed-lease break-period truncation.
- `StorageErrorFactory` for protocol errors.
- `LeaseAvailableState`, `LeaseBreakingState`, and `LeaseBrokenState` for outgoing transitions.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `class ... extends LeaseStateBase` | `enum` variants or sealed state structs | Keep immutable transition methods and constructor invariants visible instead of collapsing lease logic into ad-hoc helpers. |
| `Context.startTime: Date` | `chrono::DateTime<Utc>` or `SystemTime` | All timer/expiry behavior is evaluated against the request timestamp, not wall-clock background jobs. |
| `uuid()` / `proposedLeaseId?: string` | `Uuid` + `Option<String>` | Preserve caller-supplied IDs and only auto-generate when TS falls back to `uuid()`. |

## Special handling
- `LeaseLeasedState.ts:18-85` accepts only `Leased` + `Locked` input with a defined `leaseId`, defined `leaseDurationType`, no `leaseBreakTime`, and a future `leaseExpireTime` if one exists.
- `LeaseLeasedState.ts:88-128` treats `acquire()` as a lease-refresh path only when `proposedLeaseId === current leaseId`; otherwise it throws `LeaseAlreadyPresent`.
- `LeaseLeasedState.ts:131-219` encodes the core break-timer logic. Fixed leases use `minDate(existingExpiry, startTime + breakPeriod)` and validate `breakPeriod` only on that branch. Infinite leases skip the `1..60` validation entirely for nonzero periods.
- `LeaseLeasedState.ts:222-249` renews fixed leases by recomputing `leaseExpireTime` from the current request timestamp, but infinite leases simply return `this` unchanged.
- `LeaseLeasedState.ts:251-273` allows `change()` when either the current lease ID or the proposed lease ID matches the stored lease ID.

## Change propagation notes
- If TS tightens break-period validation for infinite leases, revisit both `LeaseLeasedState` and the `leaseTime` values returned by blob/container break APIs.
- Any change to `change()` validation must be propagated to both blob and container lease entrypoints because they rely on this shared state machine.
