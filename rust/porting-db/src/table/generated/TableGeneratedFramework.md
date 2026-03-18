# Table Generated Framework (Phase 15.1)

## File Info
- **Source Path:** `src/table/generated/` (30 files, ~3,814 LOC)
- **Rust Target:** `azurite-table/src/generated/`
- **Type:** Generated framework (autorest + custom template)
- **Phase:** 15.1
- **Complexity:** H (high)
- **Status:** ported

## Structure Overview

**30 files across 5 categories:**
1. **Artifacts** (5 files, 3,814 LOC)
   - `models.ts` (1,661 LOC)
   - `mappers.ts` (1,254 LOC)
   - `specifications.ts` (574 LOC)
   - `parameters.ts` (298 LOC)
   - `operation.ts` (27 LOC)

2. **Framework** (7 files, ~563 LOC)
   - Context, Request/Response adapters, Middleware factories

3. **Errors** (4 files, 59 LOC)
   - Middleware error wrappers

4. **Handlers** (4 files, ~200 LOC)
   - Handler interfaces and mappers

5. **Middleware** (6 files, 562 LOC)
   - Dispatch, deserializer, serializer, error, end middleware

6. **Utils** (4 files, 466 LOC)
   - Serialization, XML, utilities, logger

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

### Artifacts (5 files, 3,814 LOC)

#### models.ts (1,661 LOC)
```rust
pub struct TableItem { ... }
pub struct TableProperties { ... }
pub struct TableEntityQueryResponse { ... }
pub struct Entity { ... }
pub struct QueryOptions { ... }
pub struct ErrorCode { ... }
pub struct TableSignedIdentifier { ... }
pub struct TableAccessPolicy { ... }
pub struct TableCorsRule { ... }
pub struct TableAnalyticsLogging { ... }
pub struct TableMetrics { ... }
pub struct TableRetentionPolicy { ... }
pub struct TableGeoReplication { ... }
// ... 15+ more model types
```

Key differences from Queue:
- **Entity model**: Complex due to EDM type system
- **Query options**: Includes OData filter, select, top parameters
- **Batch operations**: Batch request/response models

#### mappers.ts (1,254 LOC)
```rust
pub fn map_table_item(source: &Json) -> TableItem { ... }
pub fn map_entity(source: &Json, entity_type: Option<EdmType>) -> Entity { ... }
pub fn map_query_options(source: &Json) -> QueryOptions { ... }
// ... one mapper per model type
```

Key differences:
- **Entity mapper**: Must handle EDM type system
- **OData parameter mapping**: Query parameters to internal model

#### specifications.ts (574 LOC)
```rust
pub struct OperationSpec {
    pub http_method: String,
    pub path: String,
    pub parameters: Vec<ParamSpec>,
}

pub const OPERATION_SPECS: &[OperationSpec] = &[
    // Service operations
    OperationSpec { http_method: "GET", path: "/...", ... },
    OperationSpec { http_method: "POST", path: "/...", ... },
    // Table CRUD
    // Batch operations
    // Entity CRUD
];
```

#### parameters.ts (298 LOC)
```rust
pub struct ParamSpec {
    pub name: String,
    pub required: bool,
    pub edm_type: String,
}

// Query parameter specs
pub const FILTER_PARAM: ParamSpec = ...;
pub const SELECT_PARAM: ParamSpec = ...;
pub const TOP_PARAM: ParamSpec = ...;
```

#### operation.ts (27 LOC)
```rust
pub enum Operation {
    ServiceProperties,
    ServiceStats,
    TableCreate,
    TableDelete,
    TableQuery,
    EntityInsert,
    EntityUpdate,
    EntityDelete,
    EntityQuery,
    BatchProcess,
    // ... 20+ total
}
```

### Handlers (4 files)
```rust
pub trait IServiceHandler: Send + Sync { ... }
pub trait ITableHandler: Send + Sync { ... }

pub trait IHandlers: Send + Sync {
    fn service_handler(&self) -> &dyn IServiceHandler;
    fn table_handler(&self) -> &dyn ITableHandler;
}

pub mod handler_mappers {
    pub fn get_handler_operation(
        method: &str,
        path: &str,
        query_params: &Map<String, Vec<String>>,
    ) -> Result<(HandlerPath, Operation), StorageError> { ... }
}
```

### Middleware (6 files, 562 LOC)
```rust
pub struct DispatchMiddleware { ... }        // Routes requests (190 LOC)
pub struct HandlerMiddlewareFactory { ... }  // Dispatches to handler (90 LOC)
pub struct DeserializerMiddleware { ... }    // Parses request body (59 LOC)
pub struct SerializerMiddleware { ... }      // Formats response (57 LOC)
pub struct ErrorMiddleware { ... }           // Error handling (134 LOC)
pub struct EndMiddleware { ... }             // Finalize response (32 LOC)
```

### Utils (4 files, 466 LOC)
```rust
pub mod serializer {
    pub fn to_json<T>(obj: &T) -> Result<String, SerializationError> { ... }
    pub fn to_atom_entry<T>(obj: &T) -> Result<String, SerializationError> { ... }
}

pub mod xml {
    pub fn parse_xml(xml_str: &str) -> Result<XmlElement, XmlError> { ... }
}

pub mod utils {
    pub fn escape_url_parameter(param: &str) -> String { ... }
}

pub trait ILogger {
    fn log(&self, level: LogLevel, msg: &str);
}
```

## Dependencies

- Phase 5 blob patterns (middleware, request/response abstractions)
- Phase 3 authentication (SAS, Shared Key interfaces)
- Phase 15.4-15.7: EDM types (Entity, EntityProperty)
- Phase 15.9: ITableMetadataStore
- Common models and serialization infrastructure

## Type Mappings

| TypeScript | Rust Equivalent | Notes |
|-----------|-----------------|-------|
| `Entity` | Entity struct | Table entity model |
| `string` | `String` | Table names, property values |
| `number` | `i32`, `i64` | Timeouts, counts |
| `boolean` | `bool` | Flags |
| `Date` | `DateTime<Utc>` | Timestamps |
| `Map<K,V>` | `HashMap<K, V>` | Properties, headers |
| `Array<T>` | `Vec<T>` | Model lists |
| `EdmType` | `EdmType` enum | Property type |
| Interface | `pub struct` or `pub trait` | Model or handler |
| async function | `async fn` returning `Result` | I/O operations |

## Special Handling / Fidelity Flags

### Critical Patterns (Identical to Queue/Blob)
1. **Middleware order is architecture-critical**
   - Pipeline: dispatch → deserializer → handler → serializer → error → end
   - Reordering silently reroutes requests

2. **Operation enum + handler mapper coupling**
   - Handlers mapped by numeric enum value + string path
   - Handler dispatch is stringly-typed
   - Enum member reordering breaks dispatch silently

3. **Request/response wrapper quirks**
   - Same header stringification behavior
   - Error middleware suppresses body/content-type only for HEAD
   - Stream semantics unchanged from blob/queue

### Table-Specific Serialization
1. **OData JSON format**:
   - Entities serialized with `@odata.type` annotations based on AnnotationLevel
   - Collection responses use OData envelope format
   - Batch responses use multipart MIME format

2. **EDM type annotations**:
   - Each property may have type annotation (e.g., `@odata.type: "Edm.Int32"`)
   - Annotation presence depends on AnnotationLevel setting
   - System properties (Timestamp, eTag) never annotated

3. **Atom XML format (optional)**:
   - Some clients use Atom XML for requests/responses
   - Must parse/generate Atom XML with entry/content elements
   - EDM types embedded in Atom entries

4. **Batch multipart format**:
   - Batch requests/responses use multipart/mixed MIME
   - Each sub-request/response has own headers and body
   - Boundary delimiter must match Content-Type header

### Comparison to Queue Generated Framework
- **Same structure**: Identical artifacts/errors/handlers/middleware layout
- **More complex models**: Table has ~30+ models vs Queue's ~20
- **Simpler handler tree**: 2 table handlers (Service, Table) vs Queue's 4
- **EDM serialization**: Table includes type annotations; queue doesn't
- **Batch support**: Table batch is multipart; queue uses queue messages

## Change Propagation

**Autorest regeneration impact:**
- If models.ts changes, BOTH mappers.ts AND serializer.ts must update
- If specifications.ts adds operation, MUST add handler interface + operation enum + mapper
- If handler interface changes, all concrete handlers break

**EDM type system changes:**
- New property type → affects Entity mapper
- Type annotation changes → affects serialization
- Comparison operator changes → affects QueryInterpreter

**OData parameter changes:**
- New filter operators → affects QueryParser/Interpreter
- New select/top parameters → affects EntityProperty selection

## Rust Porting Notes

1. **Middleware trait objects**: Use `Box<dyn Middleware>` or trait objects; order must match TS
2. **Handler dispatch**: Implement as `match` or `HashMap` to avoid dynamic lookup
3. **Models as structs**: Use `#[derive(Serialize, Deserialize)]` with serde
4. **Request/response wrappers**: Keep exact header value conversion (stringify numbers)
5. **Operation enum**: Use numeric discriminants to match handler mappers
6. **Serializer options**: Configure JSON/XML with exact OData compliance
7. **Error suppression**: Replicate HEAD-request-specific logic
8. **EDM types**: Compose with Box<dyn IEdmType> in Entity properties
9. **OData format**: Custom serializers for @odata.type, @odata.metadata fields
10. **Batch parsing**: Multipart MIME parsing for batch requests
11. **Atom XML**: Consider xml-rs or minidom for Atom entry generation
12. **Query string**: Case-insensitive parameter parsing (filter, select, top)

## Implementation Strategy

Propose 2-file porting units for Phase 15.1:
1. **Unit 1**: Models + Mappers (data models and serialization)
2. **Unit 2**: Specifications + Parameters + Operation enum + Middleware + Framework (routing and middleware chain)

**Note:** Phase 15.4-15.7 (Entity types) must complete before Phase 15.1 serialization can be validated.
