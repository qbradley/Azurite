# Porting Record — `src/common/persistence/IOperationQueue.ts`

## File info
- Source path: `src/common/persistence/IOperationQueue.ts`
- Source lines: `18`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/persistence/i_operation_queue.rs`
- Crate: `azurite-common`
- Module: `persistence::i_operation_queue`
- Phase: `1.15`
- Status: `ported`

## Exported API
### Default interface `IOperationQueue`
- `operate<T>(op: () => Promise<T>, contextId?: string): Promise<T>`

## Dependencies
- Imports: none.
- Important implementation: `OperationQueue.ts`.
- Main consumers: `FSExtentStore` read and append queues.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| generic `T` | generic `T: Send + 'static` | Preserve result type of queued operation. |
| `() => Promise<T>` | `FnOnce() -> Future<Output = Result<T, StorageError>>` | Lazy callback is essential; caller schedules work, not a running promise. |
| `Promise<T>` | `async fn -> Result<T, StorageError>` | Operation result resolves when queued work completes. |
| `string | undefined` | `Option<&str>` / `Option<String>` | Persistence subsystem uses `contextId` casing. |

## Recommended Rust translation
```rust
#[async_trait]
pub trait OperationQueue: Send + Sync {
    async fn operate<T, F, Fut>(
        &self,
        op: F,
        context_id: Option<&str>,
    ) -> Result<T, StorageError>
    where
        T: Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, StorageError>> + Send + 'static;
}
```

## Special handling
- The generic method means a trait-object form (`dyn OperationQueue`) is not object-safe. Aragorn should expect the concrete `OperationQueue` struct to be the primary runtime type unless TS changes the abstraction.
- Preserve laziness: TS accepts a callback, not an already-started promise. That is how the queue controls concurrency.
- Keep the subsystem-local `contextId` spelling documented separately from logger `contextID`.

## Change propagation notes
- If TS widens `operate()` to accept cancellation, priority, or metadata, revisit both the trait shape and the concrete scheduler implementation.
- Any change to the callback contract affects `FSExtentStore`, which relies on queued read/write serialization.

## Rust port notes
- Ported to `rust/crates/azurite-common/src/persistence/i_operation_queue.rs` as a generic async trait.
- Forced deviation remains intentional: the generic `operate<T>()` method is not object-safe, so the Phase 1 port keeps it for concrete `OperationQueue` implementations rather than forcing a boxed trait-object abstraction.
