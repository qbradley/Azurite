# Porting Record — `src/common/persistence/OperationQueue.ts`

## File info
- Source path: `src/common/persistence/OperationQueue.ts`
- Source lines: `123`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/persistence/operation_queue.rs`
- Crate: `azurite-common`
- Module: `persistence::operation_queue`
- Phase: `2.1`
- Status: `analyzed`

## Exported API
### Default class `OperationQueue implements IOperationQueue`
- **Constructor**: `new OperationQueue(maxConcurrency: number, logger: ILogger): OperationQueue`
- **Method**: `async operate<T>(op: () => Promise<T>, contextId?: string): Promise<T>`
- **Private method**: `private async execute<T>(contextId?: string): Promise<any>` (dequeues one operation and runs it; emits result or error on EventEmitter)
- **Properties**:
  - `private operations: IOperation[]`
  - `private emitter: EventEmitter`
  - `private runningConcurrency: number`
  - `private maxConcurrency: number`
  - `private readonly logger: ILogger`

### Private interface `IOperation`
- `id: string` (UUID)
- `op: () => Promise<any>`

## Dependencies
- Imports: `events.EventEmitter`, `../ILogger`, `./IOperationQueue`, `uuid`
- Porting status:
  - `ILogger`: analyzed (`1.3`)
  - `IOperationQueue`: analyzed (`1.15`)
  - `uuid`: external crate (`uuid` v4 equivalent)
  - `EventEmitter`: replace with a Tokio-native queue/signal primitive while preserving FIFO dequeue + callback completion behavior
- Important consumers: `FSExtentStore` (appendQueue, readQueue)

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `EventEmitter` | `tokio::sync::Semaphore` + `VecDeque<Operation>` | Replace event-driven pattern with async queue. |
| `IOperation.id` | `uuid::Uuid` | Type-safe UUID. |
| `() => Promise<T>` | `FnOnce() -> Pin<Box<dyn Future<Output = Result<T, StorageError>> + Send>>` | Preserve laziness. |
| `runningConcurrency` | `Arc<AtomicUsize>` | Shared counter. |
| `operations` array | `Arc<Mutex<VecDeque<Operation>>>` | Async-aware queue. |

## Recommended Rust translation
```rust
pub struct OperationQueue {
    max_concurrency: usize,
    logger: Arc<dyn ILogger + Send + Sync>,
    operations: Arc<Mutex<VecDeque<Operation>>>,
    running_concurrency: Arc<AtomicUsize>,
    semaphore: Arc<Semaphore>,
}

#[async_trait]
impl IOperationQueue for OperationQueue {
    async fn operate<T, F, Fut>(
        &self,
        op: F,
        context_id: Option<&str>,
    ) -> Result<T, StorageError>
    where
        T: Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, StorageError>> + Send + 'static,
    {
        let _permit = self.semaphore.acquire().await;
        op().await
    }
}
```

## Special handling
- **EventEmitter-based concurrency**: Replace with `tokio::sync::Semaphore` for FIFO queueing.
- **Generic operation result**: Preserve laziness; accept callback not pre-started promise.
- **Concurrency semantics**: N concurrent operations with FIFO queue for remainder.
- **Error recovery**: Ensure semaphore permit released on error via RAII or explicit cleanup.

## Change propagation notes
- If FSExtentStore changes queue usage, revisit operation scheduling contract.
- If maxConcurrency becomes dynamic, rework semaphore setup.
- If contextId semantics change, update logging and queue routing.

