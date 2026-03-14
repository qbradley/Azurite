# Queue Handlers (Phase 14.10-14.14)

## File Info
- **Source Path:** `src/queue/handlers/` (5 files, ~1,168 LOC)
- **Rust Target:** `azurite-queue/src/handlers/`
- **Type:** Handler implementations
- **Phase:** 14.10-14.14
- **Complexity:** M (medium)
- **Status:** analyzed

## Exports

```rust
pub struct BaseHandler {
    protected_metadata_store: Arc<dyn IQueueMetadataStore>,
    protected_extent_store: Arc<dyn IExtentStore>,
    protected_logger: Arc<dyn ILogger>,
}

pub struct ServiceHandler {
    // extends BaseHandler
}

pub struct QueueHandler {
    // extends BaseHandler
}

pub struct MessagesHandler {
    // extends BaseHandler
}

pub struct MessageIdHandler {
    // extends BaseHandler
}
```

## Detailed Exports by Handler

### BaseHandler (19 LOC)
```rust
pub struct BaseHandler {
    metadata_store: Arc<dyn IQueueMetadataStore>,
    extent_store: Arc<dyn IExtentStore>,
    logger: Arc<dyn ILogger>,
}

impl BaseHandler {
    pub fn new(
        metadata_store: Arc<dyn IQueueMetadataStore>,
        extent_store: Arc<dyn IExtentStore>,
        logger: Arc<dyn ILogger>,
    ) -> Self { ... }
}
```

### ServiceHandler (265 LOC) — Implements IServiceHandler
```rust
pub async fn set_properties(
    storage_service_properties: StorageServiceProperties,
    options: &ServiceSetPropertiesOptionalParams,
    context: &Context,
) -> Result<ServiceSetPropertiesResponse, StorageError> { ... }

pub async fn get_properties(
    options: &ServiceGetPropertiesOptionalParams,
    context: &Context,
) -> Result<ServiceGetPropertiesResponse, StorageError> { ... }

pub async fn get_statistics(
    options: &ServiceGetStatisticsOptionalParams,
    context: &Context,
) -> Result<ServiceGetStatisticsResponse, StorageError> { ... }

pub async fn list_queues_segment(
    options: &ServiceListQueuesSegmentOptionalParams,
    context: &Context,
) -> Result<ServiceListQueuesSegmentResponse, StorageError> { ... }
```

**Key Methods:**
- `get_properties()`: Reads service metadata (CORS, logging, retention)
- `set_properties()`: Writes service metadata; validates XML formatting
- `get_statistics()`: Returns service stats (used capacity, default replication)
- `list_queues_segment()`: Paginates queue list; supports prefix filtering and maxresults

**Validation logic:**
- CORS rules: Max rule count validation
- Retention policy: Enabled flag + days validation
- Logging: Retention + delete/read/write flag validation
- maxresults: Range 1..2147483647 (uint32 max)

**Quirks:**
- `get_properties()` includes CORS rules even if unset
- `set_properties()` must parse XML; includes default values for unspecified properties

### QueueHandler (327 LOC) — Implements IQueueHandler
```rust
pub async fn create(
    options: &QueueCreateOptionalParams,
    context: &Context,
) -> Result<QueueCreateResponse, StorageError> { ... }

pub async fn delete(
    options: &QueueDeleteOptionalParams,
    context: &Context,
) -> Result<QueueDeleteResponse, StorageError> { ... }

pub async fn get_properties(
    options: &QueueGetPropertiesOptionalParams,
    context: &Context,
) -> Result<QueueGetPropertiesResponse, StorageError> { ... }

pub async fn get_properties_with_head(
    options: &QueueGetPropertiesWithHeadOptionalParams,
    context: &Context,
) -> Result<QueueGetPropertiesWithHeadResponse, StorageError> { ... }

pub async fn set_metadata(
    options: &QueueSetMetadataOptionalParams,
    context: &Context,
) -> Result<QueueSetMetadataResponse, StorageError> { ... }

pub async fn get_access_policy(
    options: &QueueGetAccessPolicyOptionalParams,
    context: &Context,
) -> Result<QueueGetAccessPolicyResponse, StorageError> { ... }

pub async fn get_access_policy_with_head(
    options: &QueueGetAccessPolicyWithHeadOptionalParams,
    context: &Context,
) -> Result<QueueGetAccessPolicyWithHeadResponse, StorageError> { ... }

pub async fn set_access_policy(
    signed_identifiers: Option<Vec<SignedIdentifier>>,
    options: &QueueSetAccessPolicyOptionalParams,
    context: &Context,
) -> Result<QueueSetAccessPolicyResponse, StorageError> { ... }

// Private helper
fn parse_metadata(
    req_metadata: &HashMap<String, String>,
    headers: &HashMap<String, String>,
) -> HashMap<String, String> { ... }
```

**Key Methods:**
- `create()`: Creates queue; checks for duplicate; stores in metadata
- `delete()`: Deletes queue and all messages; no cascade delete error handling
- `get_properties()`: Returns approximate message count + metadata
- `set_metadata()`: Stores custom key-value metadata (case-preserved)
- `get/set_access_policy()`: ACL management with max 5 policies per queue
- `parse_metadata()`: Preserves original header case while extracting X-MS-META- prefixed headers

**Validation logic:**
- Queue name: alphanumeric + dashes, 3-63 chars, must not contain consecutive dashes
- Metadata: Max 2 KB per key, preserves case from wire
- ACL: Max 5 SignedIdentifier entries; permissions validated as one of "racud"
- Access policy: Permissions must be valid queue operation letters

**Quirks:**
- `parse_metadata()` uses HeaderConstants.X_MS_META prefix lookup (case-sensitive)
- Metadata case preserved from request headers exactly
- ACL XML parsing uses IAccessPolicy array pattern

### MessagesHandler (382 LOC) — Implements IMessagesHandler
```rust
pub async fn enqueue(
    queue_message: QueueMessage,
    options: &MessagesEnqueueOptionalParams,
    context: &Context,
) -> Result<MessagesEnqueueResponse, StorageError> { ... }

pub async fn peek(
    options: &MessagesPeekOptionalParams,
    context: &Context,
) -> Result<MessagesPeekResponse, StorageError> { ... }

pub async fn dequeue(
    options: &MessagesDequeueOptionalParams,
    context: &Context,
) -> Result<MessagesDequeueResponse, StorageError> { ... }

pub async fn clear(
    options: &MessagesClearOptionalParams,
    context: &Context,
) -> Result<MessagesClearResponse, StorageError> { ... }
```

**Key Methods:**
- `enqueue()`: Adds message to queue; stores text in extent store; validates size/TTL
- `peek()`: Returns message without dequeuing; doesn't change visibility timeout
- `dequeue()`: Pops message(s); generates pop-receipt; updates visibility timeout
- `clear()`: Deletes all messages from queue

**Validation logic:**
- Message text: Max 65536 bytes (64 KB) — stored in extent store, reference in queue metadata
- Message TTL: Range 1..604800 seconds (7 days); negative = infinity
- Visibility timeout: Range 0..604800 seconds (7 days); default 30s
- Dequeue count: Max 32 messages per dequeue request
- Peek count: Max 32 messages per peek request

**Async patterns:**
- `readStreamToString()`: Reads message text from extent store (async I/O)
- `extentStore.appendExtent()`: Stores message text as new extent (returns extent ID)
- `metadataStore.queryMessages()`: Queries message metadata from Loki

**Quirks:**
- Message text stored SEPARATELY from metadata in extent store
- Empty message text handled by deserializer bug workaround (manual XML parsing)
- Pop receipt is UUID generated per dequeue
- Visibility timeout stored in message metadata; checked against `context.startTime`

### MessageIdHandler (175 LOC) — Implements IMessageIdHandler
```rust
pub async fn update(
    queue_message: QueueMessage,
    pop_receipt: String,
    visibility_timeout: i32,
    options: &MessageIdUpdateOptionalParams,
    context: &Context,
) -> Result<MessageIdUpdateResponse, StorageError> { ... }

pub async fn delete(
    pop_receipt: String,
    options: &MessageIdDeleteOptionalParams,
    context: &Context,
) -> Result<MessageIdDeleteResponse, StorageError> { ... }
```

**Key Methods:**
- `update()`: Changes message text + visibility timeout; requires matching pop-receipt
- `delete()`: Removes message; requires matching pop-receipt

**Validation logic:**
- Pop receipt: Must match exactly (UUID format); invalid pop receipt = 400 error
- Message text: Same size validation as enqueue (max 65536 bytes)
- Visibility timeout: Range 0..604800 seconds

**Async patterns:**
- `extentStore.appendExtent()`: Stores updated message text
- `metadataStore.updateMessage()`: Updates message in metadata store
- Pop receipt validation before any mutation

## Dependencies

- Phase 1: IExtentStore, IDataStore
- Phase 14.1: Context, generated handler interfaces
- Phase 14.7: IQueueMetadataStore
- Phase 14.2-3: StorageErrorFactory, StorageError
- Common: ILogger, HeaderConstants, utility functions

## Type Mappings

| TypeScript | Rust Equivalent | Notes |
|-----------|-----------------|-------|
| `async function` | `async fn` returning `Result` | All handler methods |
| `Promise<Response>` | `Result<Response, StorageError>` | Return type pattern |
| `Context` | `&Context` | Request context reference |
| `IQueueMetadataStore` | `Arc<dyn IQueueMetadataStore>` | Metadata persistence |
| `IExtentStore` | `Arc<dyn IExtentStore>` | Binary data storage |
| `string` | `String` | Queue names, message IDs, pop receipts |
| `number` | `i32`, `i64` | Timeouts, counts, byte lengths |
| `Buffer` | `Vec<u8>` | Message binary content |
| `Map<string, string>` | `HashMap<String, String>` | Metadata, headers |
| `SignedIdentifier` | `SignedIdentifier` struct | ACL entries |

## Special Handling / Fidelity Flags

### Critical Message Semantics
1. **Message text storage isolation**: Message text is NOT stored in message metadata; stored separately in extent store
   - Metadata contains extent ID reference only
   - Text retrieval requires async extent store read
   - Must preserve exact extent ID format and reference semantics

2. **Pop-receipt coupling to message state**:
   - Pop receipt UUID tied to specific visibility timeout
   - Changing visibility resets pop receipt
   - Must generate new UUID on each dequeue/update
   - Invalid pop receipt fails the operation (no partial updates)

3. **Visibility timeout is lazy-evaluated**:
   - Not checked during storage; checked during dequeue/peek
   - Based on `context.startTime` vs stored expiration time
   - Expired messages remain in queue until accessed

4. **Message size includes XML encoding**:
   - 65536 byte limit is on the wire size (XML-encoded)
   - Binary content must be base64-encoded for storage
   - Exact size calculation must match TS implementation

5. **Metadata case preservation**:
   - Custom metadata header parsing preserves original case
   - Uses HeaderConstants.X_MS_META prefix lookups (case-sensitive matching on prefix)
   - Must handle header name case-insensitivity but value case-sensitivity

### Comparison to Blob Handlers
- **Simpler handler count**: 4 queue handlers vs 6+ blob handlers
- **Message-centric**: Queue operations center on message lifecycle
- **Extent store usage**: Both use extent store for binary content
- **Metadata preservation**: Both preserve custom header casing
- **Error handling**: Identical error factory pattern

## Change Propagation

**IQueueMetadataStore interface changes:**
- Any signature change in queryMessages/insertMessage/deleteMessage requires all handlers to update
- Return type changes (e.g., adding eTag field) cascade to response serialization

**StorageErrorFactory changes:**
- New error codes added → must update error handling in handlers
- Handler logic may need to generate new error types based on validation

**Extent store changes:**
- If appendExtent() signature changes, update in enqueue/update methods
- If readStreamToString() changes, update message retrieval

## Rust Porting Notes

1. **Arc<dyn Trait>**: Use for dependency injection of metadata/extent stores and logger
2. **async fn**: All handler methods are async; consider `tokio` runtime
3. **Result<Response, StorageError>**: Standard error handling for all operations
4. **Message text async I/O**: Must await extent store reads/writes
5. **UUID generation**: Use `uuid` crate for pop receipt generation
6. **Metadata parsing**: Case-preserve header values; use lowercase for key matching
7. **Validation ranges**: Use constants for max values (65536, 604800, etc.)
8. **DateTime arithmetic**: Use `chrono` for visibility timeout calculations
9. **Base64 encoding**: Message text must be base64-encoded for extent store
10. **Error messages**: Match TS error messages exactly for test compatibility

## Implementation Strategy

Propose 2-file porting units for Phase 14.10-14.14:
1. **Unit 1**: BaseHandler + ServiceHandler (shared base + service-level operations)
2. **Unit 2**: QueueHandler + MessagesHandler + MessageIdHandler (queue + message CRUD operations)
