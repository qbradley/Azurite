# Porting Record — `src/blob/lease/LeaseBrokenState.ts`

## File info
- Source path: `src/blob/lease/LeaseBrokenState.ts`
- Source lines: `219`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/lease_broken_state.rs`
- Crate: `azurite-blob`
- Module: `lease::lease_broken_state`
- Phase: `8.6`
- Status: `ported`

## Exported API
### Default class `LeaseBrokenState`
- Extends `LeaseStateBase`.
- Constructor: `new LeaseBrokenState(lease: ILease, context: Context)`
- Transition methods:
  - `acquire(duration, proposedLeaseId?) -> LeaseLeasedState`
  - `break() -> this`
  - `renew(leaseId) -> throws`
  - `change() -> throws LeaseNotPresentWithLeaseOperation`
  - `release(leaseId) -> LeaseAvailableState`

## Dependencies
- `uuid` for fresh IDs on reacquire.
- `StorageErrorFactory` for broken-lease errors.
- `LeaseAvailableState` and `LeaseLeasedState` for outgoing transitions.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `class ... extends LeaseStateBase` | `enum` variants or sealed state structs | Keep immutable transition methods and constructor invariants visible instead of collapsing lease logic into ad-hoc helpers. |
| `Context.startTime: Date` | `chrono::DateTime<Utc>` or `SystemTime` | All timer/expiry behavior is evaluated against the request timestamp, not wall-clock background jobs. |
| `uuid()` / `proposedLeaseId?: string` | `Uuid` + `Option<String>` | Preserve caller-supplied IDs and only auto-generate when TS falls back to `uuid()`. |

## Special handling
- `LeaseBrokenState.ts:23-71` accepts canonical broken leases (`Broken` + `Unlocked` + lease ID present, all timer/duration fields absent).
- `LeaseBrokenState.ts:71-136` also accepts expired `Breaking` input and normalizes it to the canonical broken payload; this is one of the lazy timer transitions in the subsystem.
- `LeaseBrokenState.ts:139-174` shares the same acquire rules as `Available`: 15-60 seconds or `-1`, TODO GUID validation, fixed expiry computed from request time.
- `LeaseBrokenState.ts:177-179` makes `break()` a true no-op instead of revalidating the request.
- `LeaseBrokenState.ts:181-190` returns a more specific `LeaseIsBrokenAndCannotBeRenewed` only when the supplied ID matches the stored lease ID.

## Change propagation notes
- If TS ever decides that broken leases should drop `leaseId`, update this constructor, `LeaseFactory`, and blob/container lease responses together.
- Reacquire behavior here must stay aligned with `Available` and `Expired` so callers see identical duration/ID semantics after auto-break or auto-expiry.
