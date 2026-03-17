# Queue Generated Framework (Phase 14.1)

## File Info
- **Source Path:** `src/queue/generated/` (31 files, ~5,859 LOC)
- **Rust Target:** `azurite-queue/src/generated/`
- **Type:** Generated framework (autorest + custom template)
- **Phase:** 14.1
- **Complexity:** H (high)
- **Status:** analyzed

## Exports

### Root Framework (7 files)
```rust
pub struct Context { ... }               // Request context with operation metadata
pub trait IRequest { ... }               // Request abstraction
pub trait IResponse { ... }              // Response abstraction
pub mod middleware_factory { ... }       // Abstract middleware factory
pub mod express_middleware_factory { ... } // Express.js adapter
pub mod express_request_adapter { ... }  // Express Request → IRequest
pub mod express_response_adapter { ... } // IResponse → Express Response
```

### Artifacts (5 files, 4,009 LOC)
```rust
// models.ts (1,674 LOC) - Data models for Queue API
pub struct AccessPolicy { ... }
pub struct QueueItem { ... }
pub struct ListQueuesSegmentResponse { ... }
pub struct QueueMessage { ... }
pub struct DequeuedMessageItem { ... }
pub struct PeekedMessageItem { ... }
pub struct StorageServiceProperties { ... }
pub struct StorageServiceStats { ... }
pub struct CorsRule { ... }
pub struct Metrics { ... }
pub struct Logging { ... }
pub struct GeoReplication { ... }
pub struct RetentionPolicy { ... }
pub struct SignedIdentifier { ... }
// + 20+ OptionalParams interfaces

// mappers.ts (1,387 LOC) - Serialization mappers
pub fn map_access_policy(source: &Json) -> AccessPolicy { ... }
pub fn map_queue_item(source: &Json) -> QueueItem { ... }
// ... one mapper per model type

// specifications.ts (632 LOC) - Operation specs and routing
pub struct OperationSpec {
    pub http_method: String,
    pub path: String,
    pub parameters: Vec<ParamSpec>,
}
pub const OPERATION_SPECS: &[OperationSpec] = &[...];

// parameters.ts (287 LOC) - Request parameter definitions
pub struct ParamSpec {
    pub name: String,
    pub required: bool,
    pub edm_type: String,
}

// operation.ts (29 LOC) - Operation enum
pub enum Operation {
    ServiceSetProperties,
    ServiceGetProperties,
    ServiceGetStatistics,
    ServiceListQueuesSegment,
    QueueCreate,
    QueueDelete,
    QueueGetProperties,
    // ... 20+ total
}
```

### Handlers (6 files, 243 LOC)
```rust
// Generated handler interfaces
pub trait IServiceHandler: Send + Sync { ... }
pub trait IQueueHandler: Send + Sync { ... }
pub trait IMessagesHandler: Send + Sync { ... }
pub trait IMessageIdHandler: Send + Sync { ... }

pub trait IHandlers: Send + Sync {
    fn service_handler(&self) -> &dyn IServiceHandler;
    fn queue_handler(&self) -> &dyn IQueueHandler;
    fn messages_handler(&self) -> &dyn IMessagesHandler;
    fn message_id_handler(&self) -> &dyn IMessageIdHandler;
}

pub mod handler_mappers {
    // Maps request paths/methods to handler dispatch
    pub fn get_handler_operation(
        method: &str,
        path: &str,
        query_params: &Map<String, Vec<String>>,
    ) -> Result<(HandlerPath, Operation), StorageError> { ... }
}
```

### Middleware (6 files, 562 LOC)
```rust
pub struct DispatchMiddleware { ... }    // Routes requests to handler (190 LOC)
pub struct HandlerMiddlewareFactory { ... } // Dispatches to correct handler (90 LOC)
pub struct DeserializerMiddleware { ... } // Parses request body to models (59 LOC)
pub struct SerializerMiddleware { ... }  // Serializes response to XML/JSON (57 LOC)
pub struct ErrorMiddleware { ... }       // Handles framework errors (134 LOC)
pub struct EndMiddleware { ... }         // Finalizes response (32 LOC)
```

### Utils (4 files, 466 LOC)
```rust
pub mod serializer {       // Object → XML/JSON serialization (392 LOC)
    pub fn to_xml<T>(obj: &T) -> Result<String, SerializationError> { ... }
    pub fn to_json<T>(obj: &T) -> Result<String, SerializationError> { ... }
}

pub mod xml {              // XML parsing utilities (41 LOC)
    pub fn parse_xml(xml_str: &str) -> Result<XmlElement, XmlError> { ... }
}

pub mod utils {            // General utilities (20 LOC)
    pub fn validate_header_name(name: &str) -> Result<(), ValidationError> { ... }
}

pub trait ILogger {        // Logger interface (13 LOC)
    fn log(&self, level: LogLevel, msg: &str);
}
```

## Dependencies

- Phase 5 blob patterns (middleware, request/response abstractions)
- Phase 3 authentication (SAS, Shared Key interfaces)
- Common models and serialization infrastructure
- IQueueMetadataStore (14.7)
- IExtentStore (Phase 1/2)

## Type Mappings

| TypeScript | Rust Equivalent | Notes |
|-----------|-----------------|-------|
| `string` | `String` | Queue names, property names |
| `number` | `i32`, `i64` | Message counts, timeouts |
| `boolean` | `bool` | Flags in properties |
| `Date` | `DateTime<Utc>` | Queue timestamps |
| `Map<K,V>` | `HashMap<K, V>` or `BTreeMap<K, V>` | Query params, headers |
| `Array<T>` | `Vec<T>` | Models with lists (CorsRules, etc.) |
| `any` | Dynamic JSON value | For untyped response fields |
| Interface | `pub struct` or `pub trait` | Model or handler contract |
| async function | `async fn` returning `Result` | All I/O operations |

## Special Handling / Fidelity Flags

### Critical Patterns
1. **Middleware order is architecture-critical**
   - Pipeline: dispatch → deserializer → handler → serializer → error → end
   - Handler invokes deserializer internally (NOT middleware-driven)
   - Reordering silently reroutes requests

2. **Operation enum + handler mapper coupling**
   - Handlers mapped by numeric enum value + string path combination
   - Handler dispatch is stringly-typed: `(handlers as any)[handlerPath.handler]()`
   - Enum member reordering breaks dispatch silently

3. **Handler dispatch is index-based + dynamic string lookup**
   - `handlerMappers.ts` maps operation path → handler name (string)
   - Handler layer then does dynamic lookup: `this.handlers[handlerName][operationName]()`
   - Not a refactoring candidate; must preserve exact string names

4. **Request/response wrapper quirks**
   - `IResponse.setHeader()` stringifies numbers/booleans
   - Error middleware suppresses body/content-type only for HEAD requests
   - Stream `.close()` semantics differ from `.end()` (keep as-is)

5. **XML generation coupled to xml2js options**
   - Models.ts uses exact xml2js options: `explicitArray: true`, `charKey: '$'`, `rootName` precision
   - Serialization must preserve sequence wrapping/unwrapping
   - Any schema change in models affects serializer behavior

6. **Header name casing is preserved**
   - Custom header parsing preserves case from wire
   - HeaderConstants used for X-MS-* prefix lookups (case-sensitive)

### Comparison to Blob Phase 5
- **Identical structure**: Same generated patterns (models, mappers, specs, handlers, middleware)
- **Fewer model types**: Queue has ~20 models vs Blob's 40+
- **Simpler handler tree**: 4 queue handlers vs 6+ blob handlers
- **Identical middleware order**: Same 6-stage pipeline
- **Same error types**: 4 middleware error wrappers

## Change Propagation

**Autorest regeneration impact:**
- If models.ts changes, BOTH mappers.ts AND serializer.ts must update
- If specifications.ts adds operation, MUST add handler interface + operation enum member + handler_mappers routing
- If handler interface changes signature, all concrete handlers (ServiceHandler, QueueHandler, etc.) break

**Handler implementation impact:**
- Changes to handler_mappers routing immediately affect request dispatch
- Changing handler method signatures requires updates in both:
  1. Generated handler interface (handlerMappers.ts)
  2. Concrete handler implementation

**Metadata preservation:**
- Queue-specific: Custom header parsing in handlers preserves original case
- Must use HeaderConstants for X-MS-META prefix lookups (case-insensitive)

## Rust Porting Notes

1. **Middleware trait objects**: Use `Box<dyn Middleware>` or trait objects; order must be exact
2. **Handler dispatch**: Implement as `match` statement or `HashMap<String, Box<dyn HandlerFn>>` to avoid dynamic lookup
3. **Models as structs**: Use `#[derive(Serialize, Deserialize)]` with serde for JSON/XML
4. **Request/response wrappers**: Keep exact header value conversion logic (stringify numbers/booleans)
5. **Operation enum**: Use numeric discriminants to match handler mappers indexing
6. **Serializer options**: Configure xml2js equivalent (likely quick-xml or minidom) with exact same settings
7. **Error suppression**: Replicate HEAD-request-specific error body suppression logic exactly
8. **String header lookups**: Case-preserve header names; use lowercase for equality checks (HTTP header names are case-insensitive by spec)

## 2026-03-17 Queue ACL + Pagination Parity Notes

- Queue XML output must preserve **mapper property order**, not `BTreeMap` key order. `SignedIdentifier` must serialize as `Id` then `AccessPolicy`, and `AccessPolicy` itself must stay `Start`, `Expiry`, `Permission` to match the TypeScript/xml2js wire format.
- Queue XML serialization must honor `xmlIsAttribute` for list responses. `EnumerationResults` needs `ServiceEndpoint` as an XML attribute, not as a child element.
- Queue list responses must build `ServiceEndpoint` from the incoming `Host` header when present so path-style local endpoints keep their port (`127.0.0.1:PORT`), matching `ExpressRequestAdapter.getEndpoint()` in TypeScript.
