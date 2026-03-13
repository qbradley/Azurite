# Porting Record — `src/common/IGCManager.ts`

## File info
- Source path: `src/common/IGCManager.ts`
- Source lines: `23`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/i_gc_manager.rs`
- Crate: `azurite-common`
- Module: `i_gc_manager`
- Phase: `1.10`
- Status: `analyzed`

## Exported API
### Default interface `IGCManager`
- `start(): Promise<void>`
- `close(): Promise<void>`

## Dependencies
- Imports: none.
- Implementations: `BlobGCManager`, `QueueGCManager`.
- Typical consumers: service server classes that own a GC manager instance.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `Promise<void>` | `async fn -> Result<(), StorageError>` | Startup and shutdown can fail. |

## Recommended Rust translation
```rust
#[async_trait]
pub trait GcManager: Send + Sync {
    async fn start(&mut self) -> Result<(), StorageError>;
    async fn close(&mut self) -> Result<(), StorageError>;
}
```

## Special handling
- Keep this separate from `DataStore`. GC managers run service loops and own background work, not just storage lifecycle.
- The TS implementations coordinate event emitters, provider initialization, and extent deletion. Preserve explicit `start()` rather than implicitly starting in constructors.

## Change propagation notes
- If TS adds state-query methods here, update all GC manager implementations and server lifecycle orchestration.
- Re-check interactions with `IGCExtentProvider` and `IExtentStore` whenever GC startup sequencing changes.
