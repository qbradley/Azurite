# LokiQueueMetadataStore (Phase 14.8)

## File Info
- **Source Path:** `src/queue/persistence/LokiQueueMetadataStore.ts` (881 LOC)
- **Rust Target:** `azurite-queue/src/persistence/loki_queue_metadata_store.rs`
- **Type:** Loki-backed metadata persistence
- **Phase:** 14.8
- **Complexity:** H (high)
- **Status:** analyzed

## Exports

```rust
pub struct LokiQueueMetadataStore {
    db: Arc<Mutex<LokiDatabase>>,
    queues_collection: Arc<Mutex<LokiCollection>>,
    messages_collection: Arc<Mutex<LokiCollection>>,
    transactions_collection: Arc<Mutex<LokiCollection>>,
}

impl IQueueMetadataStore for LokiQueueMetadataStore {
    // All 21 methods from IQueueMetadataStore
}

impl ICleaner for LokiQueueMetadataStore {
    async fn clearAsync(&self) -> Result<(), PersistenceError>;
}
```

## Detailed Method Signatures

### Queue Operations
```rust
pub async fn create_queue(
    &self,
    context: &Context,
    queue_model: &Queue,
) -> Result<(), QueueError> { ... }

pub async fn query_queues(
    &self,
    context: &Context,
    account: &str,
    options: &QueryOptions,
    next_marker: Option<&str>,
) -> Result<(Vec<Queue>, Option<String>), QueueError> { ... }

pub async fn delete_queue(
    &self,
    context: &Context,
    queue: &Queue,
    account: &str,
) -> Result<(), QueueError> { ... }

pub async fn get_queue(
    &self,
    account: &str,
    queue: &str,
    context: &Context,
) -> Result<Queue, QueueError> { ... }

pub async fn set_queue_acl(
    &self,
    account: &str,
    queue: &str,
    context: &Context,
    acl: Option<&Vec<SignedIdentifier>>,
) -> Result<(), QueueError> { ... }
```

### Message Operations
```rust
pub async fn query_messages(
    &self,
    context: &Context,
    account: &str,
    queue: &str,
    options: &QueryOptions,
) -> Result<Vec<QueueMessage>, MessageError> { ... }

pub async fn insert_message(
    &self,
    context: &Context,
    account: &str,
    queue: &str,
    message: &QueueMessage,
) -> Result<QueueMessage, MessageError> { ... }

pub async fn dequeue_messages(
    &self,
    context: &Context,
    account: &str,
    queue: &str,
    count: u32,
    visibility_timeout: i32,
) -> Result<Vec<DequeuedMessage>, MessageError> { ... }

pub async fn peek_messages(
    &self,
    context: &Context,
    account: &str,
    queue: &str,
    count: u32,
) -> Result<Vec<PeekedMessage>, MessageError> { ... }

pub async fn update_message(
    &self,
    context: &Context,
    account: &str,
    queue: &str,
    message_id: &str,
    message: &QueueMessage,
    pop_receipt: &str,
) -> Result<QueueMessage, MessageError> { ... }

pub async fn delete_message(
    &self,
    context: &Context,
    account: &str,
    queue: &str,
    message_id: &str,
    pop_receipt: &str,
) -> Result<(), MessageError> { ... }
```

### Batch Operations
```rust
pub async fn begin_batch_transaction(
    &self,
    account: &str,
    queue: &str,
    batch_id: &str,
) -> Result<(), BatchError> { ... }

pub async fn commit_batch_transaction(
    &self,
    batch_id: &str,
) -> Result<(), BatchError> { ... }

pub async fn abort_batch_transaction(
    &self,
    batch_id: &str,
) -> Result<(), BatchError> { ... }
```

## Data Model

### Queue Storage Model
```rust
pub struct QueueModel {
    pub account: String,
    pub queue_name: String,
    pub properties: QueueProperties,
    pub signed_identifiers: Option<Vec<SignedIdentifier>>,
    pub etag: String,
    pub last_modified: DateTime<Utc>,
    pub approximate_message_count: u32,
    pub metadata: HashMap<String, String>,
}

pub struct QueueProperties {
    pub queue_encryption: Option<QueueEncryption>,
    pub approximate_message_count: u32,
}
```

### Message Storage Model
```rust
pub struct MessageModel {
    pub message_id: String,
    pub queue_name: String,
    pub account: String,
    pub message_text: String,  // Stored as-is; also stored in extent store
    pub pop_receipt: String,
    pub visibility_timeout_at: Option<DateTime<Utc>>,
    pub time_next_visible: Option<DateTime<Utc>>,
    pub expiration_time: DateTime<Utc>,
    pub insertion_time: DateTime<Utc>,
    pub dequeue_count: u32,
}
```

### Transaction Storage Model
```rust
pub struct TransactionModel {
    pub batch_id: String,
    pub account: String,
    pub queue_name: String,
    pub status: String,  // "begun" | "committed" | "aborted"
    pub operations: Vec<OperationRecord>,
}
```

## Dependencies

- Phase 1: IQueueMetadataStore, ICleaner, IDataStore
- Phase 2: LokiDatabase, LokiCollection (from common persistence)
- Phase 14.1: Context
- Common: DateTime utilities, UUID generation

## Type Mappings

| TypeScript | Rust Equivalent | Notes |
|-----------|-----------------|-------|
| `Queue` model | `QueueModel` struct | Queue metadata |
| `QueueMessage` | `MessageModel` struct | Message data |
| `SignedIdentifier[]` | `Vec<SignedIdentifier>` | ACL entries |
| `Map<string, string>` | `HashMap<String, String>` | Metadata fields |
| `Date` | `DateTime<Utc>` | Timestamps |
| `Promise<Queue[]>` | `Result<Vec<Queue>, Error>` | Async result |
| `Loki.Collection` | `Arc<Mutex<LokiCollection>>` | Shared DB collection |
| `number` | `u32`, `i32`, `i64` | Counters, timeouts |

## Special Handling / Fidelity Flags

### Critical Queue Metadata Patterns
1. **Three-collection model**: Queues + Messages + Transactions
   - `queues`: Stores queue metadata (name, properties, ACL)
   - `messages`: Stores message records (including text copy)
   - `transactions`: Stores batch transaction state

2. **Queue name is primary key**:
   - No duplicate queue creation within account
   - Queue deletion cascades message deletion (must clear messages first)
   - Queue listing supports prefix filtering + continuation token

3. **Message text stored twice**:
   - Once in `messages` collection (for quick queries)
   - Once in extent store (for large message handling)
   - Sync between the two is critical; must match exactly

4. **Visibility timeout is lazy-evaluated**:
   - `time_next_visible` field stored in message
   - Dequeued messages have `time_next_visible` set to future time
   - Peek ignores visibility; dequeue checks time_next_visible
   - Must compare against `context.startTime` (request time), not current time

5. **Pop receipt is UUID generated per dequeue**:
   - Changes on every dequeue/update operation
   - Must match exactly for update/delete operations
   - Invalid pop receipt → 400 error

6. **Batch transactions are isolated**:
   - `beginBatchTransaction()` creates transaction record
   - All operations within batch use `batch_id`
   - `commitBatchTransaction()` applies all operations atomically
   - `abortBatchTransaction()` rolls back transaction

### Message Expiration Semantics
- `expiration_time`: When message auto-deletes from queue
- `time_next_visible`: When message becomes visible again after dequeue
- Expired messages remain in queue until accessed
- No background cleanup; cleanup happens on dequeue/peek

### Query Options Filtering
- Prefix filtering on queue name (for `queryQueues()`)
- Numeric limit on message count (for `queryMessages()`)
- Continuation token support for pagination

### Comparison to Blob Persistence (Phase 10)
- **Data model**: Similar Loki-based storage
- **Extent coupling**: Both store binary data separately in extent store
- **Query patterns**: Similar collection-based filtering
- **Lazy evaluation**: Both use lazy expiration models
- **Async patterns**: Both async/await throughout

## Change Propagation

**Queue creation/deletion cascades:**
- Creating queue with duplicate name → error (no replacement)
- Deleting queue must also delete all messages in batch (atomic)
- Metadata changes (ACL, properties) update queue record only

**Message updates impact:**
- Changing `time_next_visible` requires updating visibility timeout
- Updating message text requires updating both collection AND extent store
- Pop receipt changes on every update (new UUID)

**Batch transaction impact:**
- Operations within batch must be rolled back on abort
- Multiple messages within same batch may share transaction context
- Batch ID is primary key for transaction isolation

## Rust Porting Notes

1. **Loki collections**: Use `Arc<Mutex<Collection>>` for shared access
2. **Async all the way**: All methods are async; use `tokio`
3. **UUID generation**: Use `uuid` crate for message IDs and pop receipts
4. **DateTime handling**: Use `chrono` for Utc timestamps
5. **HashMap for metadata**: Preserve case of custom metadata fields
6. **Query filtering**: Implement prefix matching for queue names
7. **Continuation tokens**: Store query state in token for resumption
8. **Atomic batch operations**: Lock collection during transaction execution
9. **Message text sync**: Update both Loki collection AND extent store
10. **Error types**: Define QueueError, MessageError, BatchError variants
11. **Context usage**: Extract request time from context for visibility checks
12. **Dequeue count**: Increment on each dequeue; reset to 0 on successful dequeue

## Implementation Strategy

Propose 2-file porting units for Phase 14.8-14.9:
1. **Unit 1**: LokiQueueMetadataStore (all queue metadata operations)
2. **Unit 2**: QueueReferredExtentsAsyncIterator (extent garbage collection)

**Note:** LokiQueueMetadataStore is tightly coupled to IQueueMetadataStore (Phase 14.7). Ensure interface is fully ported before implementing this store.
