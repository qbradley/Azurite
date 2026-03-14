# Porting Record — `src/blob/lease/BlobWriteLeaseSyncer.ts`

## File info
- Source path: `src/blob/lease/BlobWriteLeaseSyncer.ts`
- Source lines: `50`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/blob_write_lease_syncer.rs`
- Crate: `azurite-blob`
- Module: `lease::blob_write_lease_syncer`
- Phase: `8.15`
- Status: `ported`

## Exported API
### Default class `BlobWriteLeaseSyncer`
- Implements `ILeaseSyncer<BlobModel>`.
- Constructor: `new BlobWriteLeaseSyncer(blob: BlobModel)`
- Method: `sync(lease: ILease): BlobModel`
- Specialized syncer used by blob write paths that should clear expired/broken leases.

## Dependencies
- `BlobModel` from `../persistence/IBlobMetadataStore`.
- `LeaseStateType` and `LeaseStatusType` from generated models.
- `ILease` / `ILeaseSyncer` from `./ILeaseState`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `ILeaseSyncer<BlobModel>` | specialized blob-write syncer trait | Keep this separate from `BlobLeaseSyncer` because write paths have extra normalization rules. |
| `Expired` / `Broken` states | explicit post-sync cleanup branch | TS clears those leases back to `Available` when certain write operations touch the blob. |

## Special handling
- `BlobWriteLeaseSyncer.ts:6-10` documents the intended call sites: PutBlob, SetBlobMetadata, SetBlobProperties, DeleteBlob, PutBlock, PutBlockList, PutPage, AppendBlock, and destination CopyBlob.
- `BlobWriteLeaseSyncer.ts:18-25` first mirrors all fields exactly like `BlobLeaseSyncer`.
- `BlobWriteLeaseSyncer.ts:27-46` then overrides `Expired` and `Broken` states back to `Available` / `Unlocked`, clearing every lease field. This is the write-time cleanup companion to `LeaseFactory`'s lazy timer transitions.
- The else-branch reassigns the same values again, so Rust can preserve the behavior without copying the redundancy verbatim if the result is still obviously identical.

## Change propagation notes
- If Aragorn ports additional blob write operations later, verify they all route through this syncer rather than the generic one.
- Any change to the list of auto-cleared states must stay coordinated with `LeaseExpiredState`, `LeaseBrokenState`, and snapshot lease handling in `LokiBlobMetadataStore`.
