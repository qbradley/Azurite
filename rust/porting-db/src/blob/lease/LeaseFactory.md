# Porting Record — `src/blob/lease/LeaseFactory.ts`

## File info
- Source path: `src/blob/lease/LeaseFactory.ts`
- Source lines: `63`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/lease_factory.rs`
- Crate: `azurite-blob`
- Module: `lease::lease_factory`
- Phase: `8.8`
- Status: `ported`

## Exported API
### Default class `LeaseFactory`
- Static method: `createLeaseState(lease: ILease, context: Context): ILeaseState`
- Reconstructs the correct concrete state from persisted lease fields plus `context.startTime`.

## Dependencies
- `LeaseAvailableState`, `LeaseLeasedState`, `LeaseBreakingState`, `LeaseBrokenState`, and `LeaseExpiredState`.
- `../generated/artifacts/models.LeaseStateType` and `../generated/Context`.
- Called from `LokiBlobMetadataStore` helper methods before every lease-sensitive read or write.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| static factory | `match` over persisted state + timestamps | This is the lazy timer engine for the whole lease subsystem. |
| `context.startTime` gate | request timestamp input | Expiry/break transitions are recomputed on access, not scheduled in the background. |
| `ILease -> ILeaseState` | enum/trait dispatch | Rust should reconstruct a typed state before validation or persistence write-back. |

## Special handling
- `LeaseFactory.ts:11-16` refuses to operate without `context.startTime`.
- `LeaseFactory.ts:18-23` treats both `leaseState === Available` and `leaseState === undefined` as `LeaseAvailableState`.
- `LeaseFactory.ts:25-34` lazily converts `Leased` input into `Expired` when a fixed lease expiry has already passed, while infinite leases remain `Leased` because their `leaseExpireTime` is `undefined`.
- `LeaseFactory.ts:40-50` similarly converts `Breaking` input into `Broken` once `context.startTime >= leaseBreakTime`.
- `LeaseFactory.ts:41-45` throws if a breaking lease has no `leaseBreakTime`.

## Change propagation notes
- Any Rust port that adds background timers would diverge from TS; all auto-expiry/breaking must remain factory-driven at read/update time.
- If TS introduces new lease states, this file is the central dispatch point that Aragorn must update before any store code can compile.
