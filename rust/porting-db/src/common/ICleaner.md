# Porting Record — `src/common/ICleaner.ts`

## File info
- Source path: `src/common/ICleaner.ts`
- Source lines: `3`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/i_cleaner.rs`
- Crate: `azurite-common`
- Module: `i_cleaner`
- Phase: `1.2`
- Status: `analyzed`

## Exported API
### Default interface `ICleaner`
- `clean(): Promise<void>`

## Dependencies
- Imports: none.
- Mixed into: `IAccountDataStore`, `IExtentStore`, `IExtentMetadataStore`, `ServerBase` implementations via `ICleaner` usage.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `Promise<void>` | `async fn -> Result<(), StorageError>` | Cleanup can fail in implementations such as extent stores. |

## Recommended Rust translation
```rust
#[async_trait]
pub trait Cleaner: Send + Sync {
    async fn clean(&mut self) -> Result<(), StorageError>;
}
```

## Special handling
- Keep `clean()` distinct from `close()`. The TS code treats cleanup as a separate capability, not an alias for shutdown.
- Some implementations are deliberate no-ops, while others require the resource to be closed first; do not bake in stronger semantics than the interface declares.

## Change propagation notes
- If parameters or return data are ever added to `clean()`, audit every `ICleaner` mixin and the server lifecycle hooks that call it.
- Preserve the single-method shape so future TS diffs remain easy to compare.
