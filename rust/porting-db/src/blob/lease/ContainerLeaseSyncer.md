# Porting Record — `src/blob/lease/ContainerLeaseSyncer.ts`

## File info
- Source path: `src/blob/lease/ContainerLeaseSyncer.ts`
- Source lines: `18`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/lease/container_lease_syncer.rs`
- Crate: `azurite-blob`
- Module: `lease::container_lease_syncer`
- Phase: `8.12`
- Status: `not_started`

## Exported API
### Default class `ContainerLeaseSyncer`
- Implements `ILeaseSyncer<ContainerModel>`.
- Constructor: `new ContainerLeaseSyncer(container: ContainerModel)`
- Method: `sync(lease: ILease): ContainerModel`

## Dependencies
- `ContainerModel` from `../persistence/IBlobMetadataStore`.
- `ILease` / `ILeaseSyncer` from `./ILeaseState`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `ILeaseSyncer<ContainerModel>` | syncer trait over container docs | Keep the separate container projection even though it mirrors the blob version. |
| container document mutation | mutable struct update | TS updates the live Loki document in place. |

## Special handling
- `ContainerLeaseSyncer.ts:8-16` is the container twin of `BlobLeaseSyncer`, mutating top-level timing fields plus nested `properties.lease*` members.
- Unlike blob writes, there is no special broken/expired normalization path for containers in this file.

## Change propagation notes
- Keep this file synchronized with `ContainerLeaseAdapter` and `LeaseFactory` if container lease storage layout changes.
- Do not “deduplicate” blob/container syncers if the Rust port would obscure their separate call sites.
