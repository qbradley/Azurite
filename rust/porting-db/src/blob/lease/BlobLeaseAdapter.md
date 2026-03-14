# Porting Record — `src/blob/lease/BlobLeaseAdapter.ts`

## File info
- Source path: `src/blob/lease/BlobLeaseAdapter.ts`
- Source lines: `53`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/blob_lease_adapter.rs`
- Crate: `azurite-blob`
- Module: `lease::blob_lease_adapter`
- Phase: `8.9`
- Status: `not_started`

## Exported API
### Default class `BlobLeaseAdapter`
- Implements `ILease`.
- Constructor: `new BlobLeaseAdapter(blob: BlobModel)`
- Utility: `toString(): string`
- Projects blob lease fields into the generic lease payload used by the state machine.

## Dependencies
- `../persistence/IBlobMetadataStore.BlobModel`.
- `../generated/artifacts/models` for lease enums.
- `./ILeaseState` for the generic lease contract.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `BlobModel` | blob metadata document/view struct | The adapter is a thin projection over persisted blob fields. |
| `implements ILease` | lease payload struct | Prefer an explicit conversion helper rather than sharing blob documents with the state machine directly. |
| `toString(): JSON` | `Debug` / `serde_json` helper | Useful for diagnostics if Rust keeps the same adapter layer. |

## Special handling
- `BlobLeaseAdapter.ts:18-31` silently defaults missing `blob.properties.leaseState` and `.leaseStatus` to `Available` / `Unlocked` instead of throwing; the commented-out errors show this was deliberate.
- `BlobLeaseAdapter.ts:33-39` copies lease timing data from both top-level blob fields and nested `properties` fields without validation.
- `BlobLeaseAdapter.ts:42-51` uses `JSON.stringify` over the projected lease payload for debugging.

## Change propagation notes
- If blob metadata ever stops storing lease fields split across top-level and nested properties, update this adapter and both blob syncers together.
- Do not silently reuse the stricter container adapter rules here; TS intentionally tolerates undefined blob lease state/status.
