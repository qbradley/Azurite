# Porting Record — `src/common/Mutex.ts`

## File info
- Source path: `src/common/Mutex.ts`
- Source lines: `75`
- Source type: `handwritten`
- Rust target: `azurite-common/src/mutex.rs`
- Crate: `azurite-common`
- Module: `mutex`
- Phase: `2.7`
- Status: `analyzed`

## Exported API
### Default class `Mutex` (static-only utility)
- **Static method**: `static async lock(key: string): Promise<void>` (acquires lock, waits if held)
- **Static method**: `static async unlock(key: string): Promise<void>` (releases lock, wakes next waiter)
- **Private static property**: `private static keys: {[key: string]: MutexLockStatus}`
- **Private static property**: `private static listeners: {[key: string]: Callback[]}`
- **Private static method**: `private static onUnlockEvent(key: string, handler: Callback): void`
- **Private static method**: `private static emitUnlockEvent(key: string): void`

### Private enum `MutexLockStatus`
- `LOCKED = 0`
- `UNLOCKED = 1`

### Private type `Callback`
- `(...args: any[]) => any` (void-returning callback)

## Dependencies
- Imports: none (pure TS implementation)
- Porting status: No external deps; pure synchronization primitive.
- Important consumers: Blob service, queue service (serialize metadata access or resource contention)

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| Static class + static state | `pub struct KeyMutex` + `lazy_static!` | Shared global state across tasks. |
| `MutexLockStatus` enum | `bool` or enum, wrapped in atomic/mutex | Track lock state per key. |
| `Callback` queue | `VecDeque<Box<dyn FnOnce() + Send>>` | FIFO queue of waiters. |
| `setImmediate(cb)` | `tokio::spawn()` or `tokio::task::spawn_blocking()` | Defer to task scheduler. |
| `Promise<void>` | `async fn -> Result<(), Error>` | Native async/await. |
| `Record<string, LockStatus>` | `Arc<Mutex<HashMap<String, ...>>>` | Thread-safe map. |

## Recommended Rust translation
```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::{HashMap, VecDeque};

pub struct KeyMutex {
    keys: Arc<Mutex<HashMap<String, MutexLockStatus>>>,
    listeners: Arc<Mutex<HashMap<String, VecDeque<Box<dyn FnOnce() + Send + 'static>>>>>,
}

#[derive(Clone, Copy, Debug)]
enum MutexLockStatus {
    Locked,
    Unlocked,
}

impl KeyMutex {
    pub fn new() -> Self {
        KeyMutex {
            keys: Arc::new(Mutex::new(HashMap::new())),
            listeners: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn lock(&self, key: &str) -> Result<(), StorageError> {
        loop {
            let mut keys = self.keys.lock().await;
            match keys.get(key) {
                Some(MutexLockStatus::Unlocked) | None => {
                    keys.insert(key.to_string(), MutexLockStatus::Locked);
                    drop(keys);
                    return Ok(());
                }
                Some(MutexLockStatus::Locked) => {
                    drop(keys);
                    // Register callback, wait for signal
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    {
                        let mut listeners = self.listeners.lock().await;
                        listeners.entry(key.to_string())
                            .or_insert_with(VecDeque::new)
                            .push_back(Box::new(move || { let _ = tx.send(()); }));
                    }
                    let _ = rx.await;
                }
            }
        }
    }

    pub async fn unlock(&self, key: &str) -> Result<(), StorageError> {
        {
            let mut keys = self.keys.lock().await;
            keys.remove(key);
        }
        
        // Emit unlock event for next waiter
        let mut listeners = self.listeners.lock().await;
        if let Some(queue) = listeners.get_mut(key) {
            if let Some(handler) = queue.pop_front() {
                tokio::spawn(async move {
                    handler();
                });
            }
        }
        Ok(())
    }
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_KEY_MUTEX: KeyMutex = KeyMutex::new();
}

// Or RAII lock guard:
pub struct MutexGuard {
    key: String,
    mutex: Arc<KeyMutex>,
}

impl Drop for MutexGuard {
    fn drop(&mut self) {
        // Note: cannot call async in Drop; must use sync lock or background task.
        // Consider: spawn background task to unlock, or use blocking_lock.
    }
}
```

## Special handling
- **Global static state**: TS uses class-level statics for global lock registry. Rust uses `lazy_static!` or `once_cell::sync::Lazy` for global `KeyMutex`.
- **Event-driven unlock**: TS registers callbacks; next waiter is invoked via `setImmediate()`. Rust equivalent:
  1. **tokio::sync::oneshot**: Each waiter gets a unique channel. Unlock dequeues and sends to next waiter.
  2. **tokio::sync::Notify**: Simpler but less fair (all waiters wake).
  3. **parking_lot::Mutex + Condvar**: Traditional but not async-native.
  - Phase 2 should use oneshot channels for fairness and async-friendliness.
- **No RAII lock guard**: TS requires explicit `lock()`/`unlock()` calls. Rust should ideally provide an RAII guard, but async Drop is not stable. Alternative: return a guard from `lock()` and call `unlock()` in drop via a background task.
- **Callback scheduling**: `setImmediate()` defers callback. Rust's `tokio::spawn()` achieves similar scheduling.

## Control flow notes
1. **lock(key)**:
   - Acquire `keys` lock.
   - Check if key is in map:
     - If absent or UNLOCKED: set to LOCKED, release lock, return immediately.
     - If LOCKED: release keys lock, register a oneshot waiter, await waiter channel.
   - Loop back and retry if awoken.

2. **unlock(key)**:
   - Acquire `keys` lock, remove key entry.
   - Acquire `listeners` lock.
   - Pop first waiter from key's queue.
   - Spawn background task to invoke waiter (via tokio::spawn).
   - Return.

3. **Waiter behavior**: When waiter's oneshot is sent, it resolves the await in `lock()`, which loops back and tries to acquire the lock again (now unlocked).

## Race condition analysis
- **TS**: Single-threaded; setImmediate deferral prevents race.
- **Rust**: Multi-threaded Tokio. Atomicity ensured by:
  - Mutex guards around state transitions.
  - Oneshot channels for safe signaling across tasks.
  - No lost signals because each waiter has its own channel.

## Important notes
- **FIFO fairness**: TS guarantees FIFO: `emitUnlockEvent()` pops first listener and schedules it. Rust oneshot approach preserves FIFO.
- **No double-lock protection**: Unlike Rust's guard-based approach, TS allows accidental double-lock or unlock. Document requirement: always pair lock/unlock.
- **contextId not present**: Unlike OperationQueue, this Mutex doesn't use contextId. Logging is minimal.

## Change propagation notes
- If locks need prioritization (e.g., high-priority wait first), change to priority queue in listeners.
- If locks require timeout, add cancellation token or timeout channel.
- If per-context locks are needed (e.g., per request), pass KeyMutex instance instead of static access.
- If lock contention is high, profile and consider lock-free data structures (e.g., `parking_lot` or custom CAS loop).
- If RAII guard becomes mandatory, implement async drop alternative or use scoped locking pattern.

