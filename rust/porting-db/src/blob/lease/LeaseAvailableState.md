# Porting Record — `src/blob/lease/LeaseAvailableState.ts`

## File info
- Source path: `src/blob/lease/LeaseAvailableState.ts`
- Source lines: `160`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/lease_available_state.rs`
- Crate: `azurite-blob`
- Module: `lease::lease_available_state`
- Phase: `8.3`
- Status: `not_started`

## Exported API
### Default class `LeaseAvailableState`
- Extends `LeaseStateBase`.
- Constructor: `new LeaseAvailableState(lease: ILease, context: Context)`
- Transition methods:
  - `acquire(duration, proposedLeaseId?) -> LeaseLeasedState`
  - `break() -> throws LeaseNotPresentWithLeaseOperation`
  - `renew(leaseId) -> throws LeaseIdMismatchWithLeaseOperation`
  - `change() -> throws LeaseNotPresentWithLeaseOperation`
  - `release(leaseId) -> throws LeaseIdMismatchWithLeaseOperation`

## Dependencies
- `uuid` for auto-generated lease IDs.
- `StorageErrorFactory` for lease protocol errors.
- `../generated/artifacts/models` for `LeaseStateType`, `LeaseStatusType`, and `LeaseDurationType`.
- `LeaseLeasedState` as the only successful outgoing state.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `class ... extends LeaseStateBase` | `enum` variants or sealed state structs | Keep immutable transition methods and constructor invariants visible instead of collapsing lease logic into ad-hoc helpers. |
| `Context.startTime: Date` | `chrono::DateTime<Utc>` or `SystemTime` | All timer/expiry behavior is evaluated against the request timestamp, not wall-clock background jobs. |
| `uuid()` / `proposedLeaseId?: string` | `Uuid` + `Option<String>` | Preserve caller-supplied IDs and only auto-generate when TS falls back to `uuid()`. |

## Special handling
- `LeaseAvailableState.ts:37-50` accepts a completely undefined incoming lease and preserves every field as `undefined`; it does not eagerly normalize to `Available` / `Unlocked`.
- `LeaseAvailableState.ts:53-93` enforces the canonical available invariant for non-undefined input: no lease ID, no expiry/break time, no duration, and `Unlocked` status.
- `LeaseAvailableState.ts:99-134` allows fixed durations 15-60 seconds or `-1` for infinite leases, computes fixed expiries from `context.startTime`, and leaves GUID format validation as a TODO.
- `LeaseAvailableState.ts:137-159` maps all other operations to specific StorageErrorFactory helpers rather than generic range errors.

## Change propagation notes
- Any Azure compatibility change to the initial available payload must also be reflected in `BlobLeaseAdapter`, `ContainerLeaseAdapter`, and `LeaseFactory`.
- If proposed lease ID validation is implemented upstream, re-check every acquire-capable state (`Available`, `Broken`, `Expired`, `Leased`).
