# Porting Record — `src/blob/lease/LeaseBreakingState.ts`

## File info
- Source path: `src/blob/lease/LeaseBreakingState.ts`
- Source lines: `173`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/lease_breaking_state.rs`
- Crate: `azurite-blob`
- Module: `lease::lease_breaking_state`
- Phase: `8.5`
- Status: `ported`

## Exported API
### Default class `LeaseBreakingState`
- Extends `LeaseStateBase`.
- Constructor: `new LeaseBreakingState(lease: ILease, context: Context)`
- Transition methods:
  - `acquire(duration, proposedLeaseId?) -> throws`
  - `break(breakPeriod?) -> LeaseBreakingState | LeaseBrokenState | this`
  - `renew(leaseId) -> throws`
  - `change(leaseId) -> throws`
  - `release(leaseId) -> LeaseAvailableState`

## Dependencies
- `../../common/utils/utils.minDate` for shortening an existing break deadline.
- `StorageErrorFactory` for breaking-specific error messages.
- `LeaseAvailableState` and `LeaseBrokenState` as the only successful outgoing states.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `class ... extends LeaseStateBase` | `enum` variants or sealed state structs | Keep immutable transition methods and constructor invariants visible instead of collapsing lease logic into ad-hoc helpers. |
| `Context.startTime: Date` | `chrono::DateTime<Utc>` or `SystemTime` | All timer/expiry behavior is evaluated against the request timestamp, not wall-clock background jobs. |
| `uuid()` / `proposedLeaseId?: string` | `Uuid` + `Option<String>` | Preserve caller-supplied IDs and only auto-generate when TS falls back to `uuid()`. |

## Special handling
- `LeaseBreakingState.ts:11-74` requires `Breaking` + `Locked` input with a defined `leaseId`, no expiry/duration fields, and a future `leaseBreakTime`.
- `LeaseBreakingState.ts:76-84` distinguishes error text in `acquire()`: matching the current lease ID yields `LeaseIsBreakingAndCannotBeAcquired`, while any other ID yields `LeaseAlreadyPresent`.
- `LeaseBreakingState.ts:86-127` keeps the subsystem lazy. `break(undefined)` is a no-op, `break(0)` forces immediate transition to `Broken`, and positive periods up to 60 seconds shorten the deadline with `minDate(currentBreakTime, startTime + breakPeriod)`.
- `LeaseBreakingState.ts:129-150` chooses between `LeaseIsBrokenAndCannotBeRenewed` / `LeaseIsBreakingAndCannotBeChanged` and a generic mismatch by comparing the supplied ID to the stored lease ID.
- `LeaseBreakingState.ts:153-172` still allows an explicit release during the break grace period.

## Change propagation notes
- Any change to break-period semantics must stay aligned with `LeaseLeasedState.break()` and `LeaseFactory` time-based reconstruction.
- Because this state still reports `Locked`, validators and syncers should continue to treat it as an active lease until the factory converts it to `Broken`.
