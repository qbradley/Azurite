# Porting Record — `src/blob/gc/BlobGCManager.ts`

## File info
- Source path: `src/blob/gc/BlobGCManager.ts`
- Source lines: `293`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/gc/blob_gc_manager.rs`
- Crate: `azurite-blob`
- Module: `gc::blob_gc_manager`
- Phase: `13.1`
- Status: `not_started`

## Exported API
### Default class `BlobGCManager`
- Implements `IGCManager`
- Constructor: `new BlobGCManager(referredExtentsProvider: IGCExtentProvider, allExtentsProvider: IGCExtentProvider, extentStore: IExtentStore, errorHandler: (err: Error) => void, logger: ILogger, gcIntervalInMS?: number)`
- Public getter: `status: Status`
- Public methods: `start(): Promise<void>`, `close(): Promise<void>`
- Private methods: `markSweepLoop()`, `markSweep()`, `getAllExtents()`, `sleep()`
- Enum: `Status { Initializing, Running, Closing, Closed }`

## Dependencies
- Node.js `EventEmitter` for abort/close signaling during GC loop
- `IGCExtentProvider` (Phase 10): Two instances for referred/all extent iterators
- `IGCManager` (Phase 1): Interface contract
- `IExtentStore` (Phase 1): Synchronous deletion of unreferenced extents
- `ILogger` (Phase 5 generated): Logging throughout lifecycle
- `DEFAULT_GC_INTERVAL_MS` (blob utils constants)

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `enum Status` | `enum Status` or `#[derive(Copy, Clone, PartialEq)]` | Four-state machine with explicit transitions |
| `EventEmitter` | `tokio::sync::mpsc` or crossbeam channel | Manage abort signal across async boundary |
| `Set<string>` | `HashSet<String>` | Extent ID set for mark-sweep algorithm |
| `Promise<void>` | `async fn` returning `Result<()>` | Primary async interface |
| `(err: Error) => void` | `Arc<dyn Fn(Box<dyn Error>)>` | Error callback; must be shareable across async |
| `NodeJS.Timeout` | `tokio::time::Sleep` | Interruptible timer with abort listener |

## Special handling

1. **State machine strictness** (`BlobGCManager.ts:67-83`, `146-162`)
   - Only `Closed` → `Running` and `Running` → `Closed` transitions allowed
   - Attempting invalid transitions throws immediately with descriptive error
   - Preserve the exact error message format for compatibility

2. **Event-driven abort mechanism** (`BlobGCManager.ts:169-179`)
   - `close()` emits `"abort"` event on EventEmitter
   - `markSweepLoop()` and `sleep()` listen for abort to interrupt gracefully
   - When abort triggers, promises resolve without error (not rejection)
   - Ensure Rust equivalent uses async cancellation token or similar pattern

3. **Mark-sweep algorithm quirk** (`BlobGCManager.ts:210-248`)
   - Line 232: Uses `Set.delete()` instead of marking; TODO comment flags this for future performance optimization
   - Preserve current behavior (delete from set = mark unreferenced) even though asymmetric with mark-sweep intent
   - Iteration yields chunks in batches (controlled by async iterator), not individual extents

4. **Interruptible sleep** (`BlobGCManager.ts:269-292`)
   - Custom `sleep()` wraps `setTimeout` with `timer.unref()` (line 289) to not block process exit
   - `abort` listener removes timer and resolves promise early
   - Zero-duration sleep is a no-op (returns immediately)
   - Type casting `as any` for timer suggest Node.js type system friction — not needed in Rust

5. **Initialization idempotency** (`BlobGCManager.ts:67-73`, `100-107`, `110-117`)
   - Before entering Running state, check if three providers are initialized
   - Call `init()` on each uninitialized provider and await completion
   - Idempotent: if already running or initialized, skip init and continue

6. **Background loop promise** (`BlobGCManager.ts:125-139`)
   - `start()` returns immediately after launching `markSweepLoop()` as background task (not awaited)
   - Loop attaches `.then()` and `.catch()` handlers to emit "closed" or "error" when done
   - Error during loop triggers automatic state transition to Closed and emits error event
   - This pattern ensures `start()` is non-blocking even for long-running GC

7. **Logging scope** (`BlobGCManager.ts:throughout`)
   - Every state transition, loop iteration, and significant event is logged
   - Log messages include method name and descriptive context (e.g., `BlobGCManager:start()`)
   - Preserve exact logging format for operational compatibility

## Change propagation notes
- If `IGCExtentProvider` interface changes, this manager's initialization assumes both providers implement `isInitialized()` and `init()` — verify contract stability
- If `IExtentStore.deleteExtents()` changes from synchronous to async, update `markSweep()` to await
- The TODO on line 232 (mark instead of delete) is a future optimization and should NOT be implemented during initial port — preserve current behavior
- If abort/close semantics change in blob server lifecycle, verify event flow through BlobServer → BlobGCManager still works correctly

