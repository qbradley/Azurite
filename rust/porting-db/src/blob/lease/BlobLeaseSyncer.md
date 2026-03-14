# Porting Record — `src/blob/lease/BlobLeaseSyncer.ts`

## File info
- Source path: `src/blob/lease/BlobLeaseSyncer.ts`
- Source lines: `17`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/blob_lease_syncer.rs`
- Crate: `azurite-blob`
- Module: `lease::blob_lease_syncer`
- Phase: `8.11`
- Status: `ported`

## Exported API
### Default class `BlobLeaseSyncer`
- Implements `ILeaseSyncer<BlobModel>`.
- Constructor: `new BlobLeaseSyncer(blob: BlobModel)`
- Method: `sync(lease: ILease): BlobModel`

## Dependencies
- `BlobModel` from `../persistence/IBlobMetadataStore`.
- `ILease` / `ILeaseSyncer` from `./ILeaseState`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `ILeaseSyncer<BlobModel>` | syncer trait over blob docs | A simple projection helper is enough; no async or validation lives here. |
| blob document mutation | mutable struct update | TS mutates the existing blob document in place and returns it. |

## Special handling
- `BlobLeaseSyncer.ts:7-15` writes every lease field back into the blob document, splitting duration/state/status into `blob.properties.*` and the rest into top-level fields.
- There is no expiry/broken normalization here; callers that need that behavior use `BlobWriteLeaseSyncer` instead.

## Change propagation notes
- Any new lease field must be added here and to `BlobWriteLeaseSyncer` together.
- If blob persistence stops mutating documents in place, callers in `LokiBlobMetadataStore` will need coordinated updates.
