# Porting Record — `src/queue/persistence/IQueueMetadataStore.ts`

## File info
- Source path: `src/queue/persistence/IQueueMetadataStore.ts`
- Source lines: `317`
- Source type: `interface + type definitions`
- Rust target (per `PORTING-ORDER.md`): `azurite-queue/src/persistence/i_queue_metadata_store.rs`
- Crate: `azurite-queue`
- Module: `persistence::i_queue_metadata_store`
- Phase: `14.7`
- Status: `ported`

## Exported API
### Type Definitions (10)
- `ServicePropertiesModel = Models.StorageServiceProperties & IServiceAdditionalProperties`
- `QueueModel = Models.QueueItem & IQueueAdditionalProperties & IServiceAdditionalProperties`
- `QueueACL = Models.SignedIdentifier[]`
- `IQueueMetadata = { [propertyName: string]: string }`
- `MessageModel = IMessageUpdateProperties & IMessageAdditionalProperties`
- `MessageUpdateProperties = IMessageUpdateProperties & { persistency?: IExtentChunk }`
- `IExtentChunk = { id: string; offset: number; count: number }`
- Interfaces: `IServiceAdditionalProperties`, `IQueueAdditionalProperties`, `IMessageUpdateProperties`, `IMessageAdditionalProperties`

### Default interface `IQueueMetadataStore`
- Extends: `IGCExtentProvider` (Phase 1), `IDataStore` (Phase 1), `ICleaner` (Phase 1)
- 21 methods:
  - **Service:** `updateServiceProperties()`, `getServiceProperties()`
  - **Queue Lifecycle:** `listQueues()`, `getQueue()`, `createQueue()`, `deleteQueue()`
  - **Queue Metadata:** `setQueueACL()`, `setQueueMetadata()`
  - **Message Count:** `getMessagesCount()`
  - **Message Operations:** `insertMessage()`, `peekMessages()`, `getMessages()`, `deleteMessage()`, `updateMessage()`, `clearMessages()`, `listMessages()`

## Dependencies
- `IGCExtentProvider` (Phase 1): Garbage collection provider interface
- `IDataStore` (Phase 1): Generic data store contract
- `ICleaner` (Phase 1): Cleanup/close interface
- Models from Phase 14.1 generated: `Models.StorageServiceProperties`, `Models.QueueItem`, `Models.SignedIdentifier`
- `Context` (Phase 5 generated): Request context parameter

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `interface extends IGCExtentProvider, IDataStore, ICleaner` | `pub trait` with super-traits | Multiple trait inheritance |
| `export type X = Y & Z` | `pub type X = (Y, Z)` or struct | Type aliases with intersection |
| `{ [key: string]: Type }` | `HashMap<String, Type>` or `BTreeMap` | Dynamic property maps |
| `Promise<[Type, number \| undefined]>` | `impl Future<Output = (Vec<Type>, Option<usize>)>` | Tuple return with continuation marker |
| `Date` | `chrono::DateTime<Utc>` | Timestamp values |
| Optional Context parameter | `Option<&Context>` or trait object | Request context for logging/tracking |

## Special handling
1. **Type composition** (`IQueueMetadataStore.ts:8-65`)
   - Models are composed by intersection types (TS `&` operator)
   - Each service/queue/message type extends Models + service-specific properties
   - Preserves separation: queue/message types have accountName, queue types have queueAcl
   - Must flatten these in Rust (recommendation: single struct per type, not tuple composition)

2. **Extent chunk model** (`IQueueMetadataStore.ts:61-65`)
   - Three-part reference: id (extent ID), offset (byte position), count (length)
   - Used for message persistence — points to blob/extent store
   - Same structure as blob phase — ensure consistency

3. **Multiple inheritance** (`IQueueMetadataStore.ts:75`)
   - Implements three interfaces: IGCExtentProvider (garbage collection), IDataStore (CRUD), ICleaner (lifecycle)
   - Methods are mixed from all three
   - Rust must implement all three traits or flatten into single trait

4. **Queue listing with continuation** (`IQueueMetadataStore.ts:109-114`)
   - Returns tuple: `[QueueModel[], number | undefined]`
   - Second element is marker/continuation token (undefined = no more results)
   - Used for pagination across potentially large queue lists

5. **Message operations with validation** (`IQueueMetadataStore.ts:244-252`, `265-286`)
   - `getMessages()` requires `timeNextVisible` and `popReceipt` (visibility timeout, dequeue receipt)
   - `deleteMessage()` and `updateMessage()` validate `validatingPopReceipt` (prevents concurrent deletes)
   - Pop receipt is queue-specific mechanism — preserve exact semantics

6. **Context parameter** (throughout)
   - Optional `Context` parameter in most methods (not mandatory)
   - Used by handlers to track request metadata (account, queue, message IDs)
   - Some methods (listQueues, listMessages) omit context parameter

7. **Message peek vs dequeue**
   - `peekMessages()` reads without visibility timeout
   - `getMessages()` marks messages invisible for `timeNextVisible` duration
   - Different method contracts must be preserved

## Change propagation notes
- If Models change (Phase 14.1 generated), audit type intersections
- Service-specific properties (accountName, queueAcl) are critical for permission/quota tracking — do not omit
- Continuation marker semantics (number | undefined for marker, integer for maxResults) are database-agnostic
- Pop receipt validation is security-critical for concurrent message operations — preserve exact algorithm
- If message persistence strategy changes (extent chunks), update IExtentChunk definition accordingly

