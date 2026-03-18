# Porting Record — `src/queue/gc/QueueGCManager.ts`

## File info
- Source path: `src/queue/gc/QueueGCManager.ts`
- Source lines: `253`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/gc/queue_gc_manager.rs`
- Crate: `azurite-queue`
- Module: `gc::queue_gc_manager`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `setInterval()` | `tokio::time::interval()` | Periodic GC sweep |
| `Promise<void>` | `async fn` | Async GC cycle |
| `IQueueMetadataStore` | `Arc<dyn IQueueMetadataStore>` | Metadata store ref |

## Special handling
GC manager for queue service. Periodically sweeps expired messages and cleans up extent references. Follows same pattern as blob equivalent. See `porting-db/src/blob/gc/BlobGCManager.md` for detailed fidelity notes.

Queue-specific: GC must handle message visibility timeout expiry and dequeue count limits. Messages past max dequeue count are moved to poison queue if configured.
