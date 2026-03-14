# Porting Record — `src/blob/lease/LeaseExpiredState.ts`

## File info
- Source path: `src/blob/lease/LeaseExpiredState.ts`
- Source lines: `243`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/lease_expired_state.rs`
- Crate: `azurite-blob`
- Module: `lease::lease_expired_state`
- Phase: `8.7`
- Status: `ported`

## Exported API
### Default class `LeaseExpiredState`
- Extends `LeaseStateBase`.
- Constructor: `new LeaseExpiredState(lease: ILease, context: Context)`
- Transition methods:
  - `acquire(duration, proposedLeaseId?) -> LeaseLeasedState`
  - `break() -> LeaseBrokenState`
  - `renew(...) -> LeaseLeasedState`
  - `change() -> throws LeaseNotPresentWithLeaseOperation`
  - `release(leaseId) -> LeaseAvailableState`

## Dependencies
- `uuid` for fresh IDs when reacquiring from an expired lease.
- `StorageErrorFactory` for mismatch/not-present errors.
- `LeaseAvailableState`, `LeaseBrokenState`, and `LeaseLeasedState` for outgoing transitions.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `class ... extends LeaseStateBase` | `enum` variants or sealed state structs | Keep immutable transition methods and constructor invariants visible instead of collapsing lease logic into ad-hoc helpers. |
| `Context.startTime: Date` | `chrono::DateTime<Utc>` or `SystemTime` | All timer/expiry behavior is evaluated against the request timestamp, not wall-clock background jobs. |
| `uuid()` / `proposedLeaseId?: string` | `Uuid` + `Option<String>` | Preserve caller-supplied IDs and only auto-generate when TS falls back to `uuid()`. |

## Special handling
- `LeaseExpiredState.ts:24-74` accepts canonical expired leases: `Expired` + `Unlocked`, stored `leaseId`, no expiry/break time, and a finite `leaseDurationSeconds` retained for possible renewals.
- `LeaseExpiredState.ts:75-138` also accepts expired `Leased` input and normalizes it to the canonical expired payload, dropping `leaseExpireTime` and `leaseDurationType` while keeping the original duration seconds.
- `LeaseExpiredState.ts:146-181` reacquires exactly like `Available` / `Broken`.
- `LeaseExpiredState.ts:184-197` ignores the requested `breakPeriod` and immediately returns `Broken`.
- `LeaseExpiredState.ts:199-215` is a compatibility quirk: the method signature omits the `leaseId` parameter entirely and unconditionally renews using the stored lease ID and retained duration seconds.

## Change propagation notes
- Any TS change to whether expired leases retain `leaseDurationSeconds` will affect renewal, lease response payloads, and `BlobWriteLeaseSyncer` auto-clear behavior.
- If the TS team fixes the missing `renew(leaseId)` validation, treat it as a compatibility-sensitive behavior change rather than a cleanup.
