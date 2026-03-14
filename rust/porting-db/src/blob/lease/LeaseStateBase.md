# Porting Record — `src/blob/lease/LeaseStateBase.ts`

## File info
- Source path: `src/blob/lease/LeaseStateBase.ts`
- Source lines: `29`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/lease_state_base.rs`
- Crate: `azurite-blob`
- Module: `lease::lease_state_base`
- Phase: `8.2`
- Status: `not_started`

## Exported API
### Default abstract class `LeaseStateBase`
- Implements `ILeaseState`.
- Constructor: `new LeaseStateBase(lease: ILease, context: Context)`
- Abstract transition methods:
  - `acquire(duration, proposedLeaseId?)`
  - `break(breakPeriod?)`
  - `renew(leaseId)`
  - `change(leaseId, proposedLeaseId)`
  - `release(leaseId)`
- Concrete helpers:
  - `sync<T>(syncer: ILeaseSyncer<T>): T`
  - `validate(validator: ILeaseValidator): ILeaseState`

## Dependencies
- `./ILeaseState` for the lease payload and strategy interfaces.
- `../generated/Context` for the protected request timestamp/context ID held by every concrete state.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `abstract class` with shared fields | base struct + trait default methods | The only shared behavior is storing `lease/context` plus forwarding `sync()` and `validate()`. |
| `public readonly lease` | owned lease payload | Concrete states deep-copy before calling `super`, so Rust should own the lease data, not borrow a mutable document. |
| `protected readonly context` | request context field | Keep it on the state object because every transition computes times and errors from the same request timestamp. |

## Special handling
- `LeaseStateBase.ts:22-27` makes `validate()` fluent by returning `this`; TS call sites rely on that chaining style in persistence methods.
- `LeaseStateBase.ts:22-26` does not clone before handing the payload to validators or syncers, so Rust should preserve by-reference access patterns where practical.

## Change propagation notes
- If future states need more shared helpers, add them here rather than duplicating transition boilerplate.
- Changing `validate()` or `sync()` chaining semantics would touch every lease operation in `LokiBlobMetadataStore`.
