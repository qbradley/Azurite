# Porting Record — `src/common/IDataStore.ts`

## File info
- Source path: `src/common/IDataStore.ts`
- Source lines: `39`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/i_data_store.rs`
- Crate: `azurite-common`
- Module: `i_data_store`
- Phase: `1.1`
- Status: `analyzed`

## Exported API
### Default interface `IDataStore`
- `init(): Promise<void>`
- `isInitialized(): boolean`
- `close(): Promise<void>`
- `isClosed(): boolean`

## Dependencies
- Imports: none.
- Notable dependents in Phase 1: `IAccountDataStore`, `IGCExtentProvider`, `IExtentMetadata`, `IExtentStore`, `IExtentMetadataStore`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `Promise<void>` | `async fn -> Result<(), StorageError>` | Align with strategy section 7 error model. |
| `boolean` | `bool` | Keep sync state probes separate from async lifecycle calls. |

## Recommended Rust translation
```rust
#[async_trait]
pub trait DataStore: Send + Sync {
    async fn init(&mut self) -> Result<(), StorageError>;
    fn is_initialized(&self) -> bool;
    async fn close(&mut self) -> Result<(), StorageError>;
    fn is_closed(&self) -> bool;
}
```

## Special handling
- Preserve the two-phase lifecycle shape: async `init/close` plus sync `isInitialized/isClosed` checks.
- Call sites check state before performing expensive startup and shutdown work, so the Rust translation should keep explicit state rather than deriving it lazily.

## Change propagation notes
- Any new lifecycle method here cascades into every storage and metadata trait in `src/common/`.
- If state semantics change, re-check GC managers and datastore implementations that guard `init()` and `close()` with state tests.
