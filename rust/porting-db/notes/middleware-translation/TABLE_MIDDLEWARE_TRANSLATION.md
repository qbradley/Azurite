# Table Service Middleware Translation Summary: TypeScript to Rust

## 1. TYPESCRIPT MIDDLEWARE BEHAVIORS & EDGE CASES

### 1.1 AuthenticationMiddlewareFactory.ts
**Location:** `/home/azureuser/Azurite/src/table/middleware/AuthenticationMiddlewareFactory.ts`

**Core Behavior:**
- Factory creates Express middleware that validates requests against multiple `IAuthenticator` instances
- Sequential authenticator iteration: tries each authenticator until one returns `true`
- Returns 403 AuthorizationFailure if all authenticators fail
- Wraps async operations with promise `.catch(next)` for error handling

**Key Logic:**
```typescript
- Creates TableStorageContext from res.locals
- Iterates authenticators: for(authenticator of authenticators) await authenticator.validate()
- Short-circuit on first successful authentication (returns true)
- Returns false if all fail → next(StorageErrorFactory.getAuthorizationFailure())
```

**Edge Cases:**
- Authenticator.validate() may return undefined or false (treats both as failure)
- Error during validate() is caught and passed to next() for error handler
- Context created twice (once for error, once for auth) - inefficient but not critical
- Logger uses contextID which may be undefined initially

---

### 1.2 PreflightMiddlewareFactory.ts
**Location:** `/home/azureuser/Azurite/src/table/middleware/PreflightMiddlewareFactory.ts`

**Core Behaviors:**

**A. createOptionsHandlerMiddleware()** - ErrorRequestHandler
- Intercepts OPTIONS requests only (method.toUpperCase() === MethodConstants.OPTIONS)
- Validates required CORS headers: `Origin`, `Access-Control-Request-Method`
- Fetches ServiceProperties from metadataStore by account
- Matches CORS rules: checks origin, method, headers (wildcard support with `*` suffix)
- Sets response headers on first matching rule or returns corsPreflightFailure

**B. createCorsRequestMiddleware()** - Dual mode handler
- Overloaded: `blockErrorRequest=true` → ErrorRequestHandler, `false` → RequestHandler
- Skips OPTIONS requests (passes to next)
- Fetches ServiceProperties and matches CORS rules
- Extracts response headers from: handlerResponses, specification mappers, error headers
- Handles header serialization using msRest.Serializer with generated Mappers
- Implements complex header exposure matching: simple headers + wildcard prefix rules

**Helper Methods:**
- `checkOrigin()`: comma-separated origins with case-insensitive matching, supports `*` wildcard
- `checkMethod()`: comma-separated methods, case-insensitive, supports `*`
- `checkHeaders()`: recursive matching with wildcard suffix (`*`) expansion
- `getResponseHeaders()`: Serializes headers from specification mappers + handler response
- `getExposedHeaders()`: Filter headers by CORS exposed rules with prefix matching

**Edge Cases:**
- Origin/Method headers may be undefined or non-string → returns 400 invalid CORS header
- CORS not enabled → no ServiceProperties or no cors field → corsPreflightFailure
- Multiple CORS rules: selects **first matching** (order matters)
- Header serialization complex: uses composite mappers, collection prefixes (e.g., x-ms-meta-*)
- Exposed headers: must match EXACT case for simple headers, case-insensitive for prefix rules
- Date, Connection, Transfer-Encoding always exposed regardless of spec

---

### 1.3 tableStorageContext.middleware.ts
**Location:** `/home/azureuser/Azurite/src/table/middleware/tableStorageContext.middleware.ts`

**Core Behavior:**
- Factory function returns RequestHandler, core logic in `tableStorageContextMiddleware()`
- Extracts storage context: account, tableName, partition key, row key from URL
- Validates API version (unless skipped) against ValidAPIVersions
- Handles TWO URL patterns:
  1. **Emulator style:** `http://hostname[:port]/account/table`
  2. **Production style:** `http[s]://account.hostname[:port]/table` (requires dot in hostname)
- Secondary account suffix handling: `-secondary` removed from account and auth path

**Table Name Extraction Logic (complex state machine):**
```
undefined/empty → tableName = undefined (service level)
"Tables"/"Tables()" → tableName = undefined (name in body)
"Tables('mytable')" → tableName = "mytable"
"mytable" → tableName = "mytable"
"mytable(PartitionKey='pk',RowKey='rk')" → tableName, partitionKey, rowKey extracted
  - Uses regex: /'([^']|'')*'/g to match quoted strings
  - Handles escaped quotes: '' → '
  - PLACEHOLDER substitution for dispatch pattern
"mytable()" → tableName = "mytable"
```

**Edge Cases:**
- Path decoding: decodeURIComponent() before splitting
- Hostname patterns: IP regex check, NO_ACCOUNT_HOST_NAMES set, dot detection
- Secondary suffix: removed from both account name AND authenticationPath
- GET requests for entity operations: tableSection converted to `tableName()` pattern
- Quoted string parsing complex: handles '' (escaped quotes) within keys
- Invalid patterns return 400 error in error handler
- System tables (prefix `$`) skip validation

**Context Fields Set:**
- `accept`, `startTime`, `xMsRequestID` (UUID v4)
- `isSecondary`, `account`, `tableName`, `partitionKey`, `rowKey`
- `authenticationPath`, `dispatchPattern`
- Server header: `Azurite-Table/{VERSION}`

---

### 1.4 telemetry.middleware.ts
**Location:** `/home/azureuser/Azurite/src/table/middleware/telemetry.middleware.ts`

**Core Behavior:**
- Factory creates RequestHandler wrapping `telemetryMiddleware(context, next)`
- Calls `AzuriteTelemetryClient.TraceRequest(context)` with Context
- Context built from: `res.locals`, path, ExpressRequestAdapter, ExpressResponseAdapter
- Immediate passthrough to next() - no blocking

**Edge Cases:**
- Simple passthrough: minimal error handling
- Depends on Context creation success (may throw if locals missing)

---

## 2. RUST MIDDLEWARE CONVENTIONS & REUSABLE HELPERS

### 2.1 Architecture Pattern (from azurite-blob/src/middlewares/)

**Standard Pattern:**
1. **Middleware struct**: `#[derive(Clone)]` with dependencies (logger, stores, etc.)
2. **apply() method**: async, takes `&Context`, `&GeneratedHttpRequest`, `&mut GeneratedHttpResponse`
3. **Factory struct**: `#[derive(Clone)]` with create*Middleware() methods
4. **Return types**: 
   - Auth middleware: `Result<(), StorageError>` (Err halts processing)
   - Preflight middleware: `Option<BoxedMiddlewareError>` (None = continue, Some = error)
   - Context middleware: `Result<(), StorageError>`
   - Telemetry middleware: returns void, no error

**File Structure:** `/home/azureuser/Azurite/rust/crates/azurite-blob/src/middlewares/`
- `mod.rs`: Re-exports all middleware types and helper types
- `authentication_middleware_factory.rs`: Authentication logic
- `blob_storage_context.rs`: Context extraction and validation
- `preflight_middleware_factory.rs`: CORS preflight handling
- `telemetry.rs`: Telemetry collection
- `strict_model_middleware_factory.rs`: Request validation (blob-specific)

### 2.2 Key Reusable Components

**From azurite-common/src:**
- `ILogger + Send + Sync`: Logging interface (verbose, info, debug, error)
- `Telemetry::AzuriteTelemetryClient::TraceRequest(TelemetryContext)`
- `TelemetryServiceType` enum: Blob, Queue, Table, Unknown
- Header/method constants in utils
- IP_REGEX, NO_ACCOUNT_HOST_NAMES from utils

**From azurite-blob/src:**
- `StorageError` struct: statusCode, headers (BTreeMap), body, contentType, storageErrorCode/Message
- `StorageErrorFactory`: static methods like getAuthorizationFailure(), corsPreflightFailure()
- `BlobStorageContext`: Wrapper around Context with account(), container(), etc. getters/setters
- `GeneratedHttpRequest/Response` traits: getMethod(), getHeader(), setHeader(), getPath()
- `IAuthenticator` trait: `async fn validate(&self, req, context) -> Result<Option<bool>>`
- `ResponseHeaderValue` enum: Single(String) | Multi(Vec<String>)
- `IBlobMetadataStore`: getServiceProperties(context, account) -> Option<ServicePropertiesModel>

**Common Patterns:**
```rust
// Type aliases
pub type SharedAuthenticator = Arc<dyn IAuthenticator + Send + Sync>;
pub type SharedMiddlewareLogger = Arc<dyn ILogger + Send + Sync>;
pub type BoxedMiddlewareError = Box<dyn std::error::Error + Send + Sync>;

// Error handling in middleware
let properties = match metadataStore.getServiceProperties(context, &account).await {
    Ok(props) => props,
    Err(error) => return Some(Box::new(error)),
};

// Optional handling
let Some(properties) = properties else {
    return Some(Box::new(StorageErrorFactory::corsPreflightFailure(...)));
};
```

---

## 3. TARGET CRATE STRUCTURE & FILES TO CREATE/EDIT

### 3.1 Current State
**Location:** `/home/azureuser/Azurite/rust/crates/azurite-table/src/`

**Existing Modules:**
```
├── authentication/         ← IAuthenticator, table-specific authenticators
├── batch/                  ← Batch operation handling
├── context/
│   ├── mod.rs
│   └── table_storage_context.rs  ← TableStorageContext (similar to BlobStorageContext)
├── entity/                 ← EDM type definitions
├── errors/                 ← StorageError, StorageErrorFactory
├── generated/              ← Generated code (Context, Request, Response, etc.)
├── handlers/               ← Operation handlers
├── middlewares/            ← **CURRENT: only mod.rs exists**
├── persistence/            ← ITableMetadataStore, ServicePropertiesModel
└── utils/                  ← Constants, utilities
```

**Current middlewares/mod.rs:**
```rust
#[derive(Debug, Clone, Default)]
pub struct TableMiddlewaresModule;
```
*Empty! Ready for implementation.*

### 3.2 Files to Create

```
/home/azureuser/Azurite/rust/crates/azurite-table/src/middlewares/
├── mod.rs                                    [EDIT - add re-exports]
├── authentication_middleware_factory.rs      [CREATE]
├── preflight_middleware_factory.rs           [CREATE]
├── table_storage_context.rs                  [CREATE]
└── telemetry.rs                              [CREATE]
```

### 3.3 Specific File Paths & Modules

**1. authentication_middleware_factory.rs**
```rust
pub type SharedAuthenticator = Arc<dyn IAuthenticator + Send + Sync>;
pub type SharedMiddlewareLogger = Arc<dyn ILogger + Send + Sync>;

#[derive(Clone)]
pub struct AuthenticationMiddleware {
    logger: SharedMiddlewareLogger,
    authenticators: Arc<Vec<SharedAuthenticator>>,
}

impl AuthenticationMiddleware {
    pub async fn apply(
        &self,
        context: &TableStorageContext,
        req: &GeneratedHttpRequest,
        res: &GeneratedHttpResponse,
    ) -> Result<(), StorageError>;
}

#[derive(Clone)]
pub struct AuthenticationMiddlewareFactory { ... }
```

**2. preflight_middleware_factory.rs**
```rust
pub type SharedPreflightLogger = Arc<dyn ILogger + Send + Sync>;
pub type SharedTableMetadataStore = Arc<dyn ITableMetadataStore + Send + Sync>;
pub type BoxedMiddlewareError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone)]
pub struct OptionsHandlerMiddleware { ... }

#[derive(Clone)]
pub struct CorsRequestMiddleware { ... }

#[derive(Clone)]
pub struct PreflightMiddlewareFactory {
    logger: SharedPreflightLogger,
}

impl PreflightMiddlewareFactory {
    pub async fn apply_options(...) -> Option<BoxedMiddlewareError>
    pub async fn apply_cors_request(...) -> Option<BoxedMiddlewareError>
}
```

**3. table_storage_context.rs**
```rust
#[allow(non_snake_case)]
pub fn createTableStorageContextMiddleware(
    skipApiVersionCheck: Option<bool>,
    disableProductStyleUrl: Option<bool>,
) -> TableStorageContextMiddlewareOptions;

#[allow(non_snake_case)]
pub fn tableStorageContextMiddleware(
    context: &Context,
    req: &GeneratedHttpRequest,
    res: &mut GeneratedHttpResponse,
    logger: &(dyn ILogger + Send + Sync),
    options: TableStorageContextMiddlewareOptions,
) -> Result<(), StorageError>;

pub fn extractStoragePartsFromPath(
    hostname: &str,
    path: &str,
    disableProductStyleUrl: bool,
) -> (Option<String>, Option<String>, bool);
```

**4. telemetry.rs**
```rust
#[allow(non_snake_case)]
pub fn telemetryMiddleware(context: &Context);

#[derive(Clone)]
pub struct TelemetryMiddleware { ... }

#[derive(Clone)]
pub struct TelemetryMiddlewareFactory { ... }
```

**5. mod.rs [EDIT]**
```rust
pub mod authentication_middleware_factory;
pub mod preflight_middleware_factory;
pub mod table_storage_context;
pub mod telemetry;

pub use authentication_middleware_factory::{
    AuthenticationMiddleware, AuthenticationMiddlewareFactory, SharedAuthenticator,
    SharedMiddlewareLogger,
};
pub use preflight_middleware_factory::{
    BoxedMiddlewareError, CorsRequestMiddleware, OptionsHandlerMiddleware,
    PreflightMiddlewareFactory, SharedTableMetadataStore, SharedPreflightLogger,
};
pub use table_storage_context::{
    tableStorageContextMiddleware, createTableStorageContextMiddleware,
    extractStoragePartsFromPath, TableStorageContextMiddlewareOptions,
};
pub use telemetry::{telemetryMiddleware, TelemetryMiddleware, TelemetryMiddlewareFactory};
```

---

## 4. INTEGRATION POINTS & MISSING TYPES

### 4.1 Dependencies Required

**From azurite-common:**
- `ILogger` trait with verbose(), info(), debug(), error() methods
- `IP_REGEX` pattern for hostname validation
- `NO_ACCOUNT_HOST_NAMES: HashSet<&str>` for special hostnames

**From azurite-table/context:**
- `TableStorageContext`: Already exists at `/home/azureuser/Azurite/rust/crates/azurite-table/src/context/table_storage_context.rs`
- Public getters/setters: account(), tableName(), partitionKey(), rowKey(), authenticationPath(), isSecondary(), accept()

**From azurite-table/authentication:**
- `IAuthenticator` trait: `async fn validate(&self, req: &GeneratedHttpRequest, context: &TableStorageContext) -> Result<Option<bool>, StorageError>`

**From azurite-table/persistence:**
- `ITableMetadataStore` trait: `async fn getServiceProperties(&self, context: &Context, account: &str) -> Result<Option<ServicePropertiesModel>, StorageError>`
- `ServicePropertiesModel`: has `accountName` and `properties: GeneratedObject` fields

**From azurite-table/errors:**
- `StorageErrorFactory`: static methods getAuthorizationFailure(), corsPreflightFailure(), getInvalidCorsHeaderValue()
- `StorageError`: error type with statusCode, headers, body fields

**From azurite-table/generated:**
- `Context` struct with storage and retrieval methods
- `GeneratedHttpRequest` trait: getMethod(), getHeader(), getPath(), getHostname()
- `GeneratedHttpResponse` trait: setHeader(), setStatusCode(), getHeader()
- `ResponseHeaderValue` enum: Single(String) | Multi(Vec<String>)

### 4.2 CRITICAL MISSING/INCOMPLETE TYPES

**1. TableStorageContext Methods**
   - Need verify all setters exist: setAccount(), setTableName(), setPartitionKey(), setRowKey(), etc.
   - May need: setDispatchPattern(), setStartTime(), setRequestID()

**2. Table-Specific Errors**
   - Verify `StorageErrorFactory::getInvalidCorsHeaderValue()` exists in table errors
   - If not, needs to be created or borrowed from blob factory

**3. ServicePropertiesModel Structure**
   - Need CORS field extraction: properties.cors -> Vec of CORS rules
   - Each CORS rule must expose: allowedOrigins, allowedMethods, allowedHeaders, maxAgeInSeconds, exposedHeaders

**4. Header Matching Utilities**
   - Need generalized checkOrigin(), checkMethod(), checkHeaders() functions
   - May extract to azurite-common or keep table-local
   - Wildcard matching logic: `*` suffix for headers, `*` full value for origins/methods

**5. GeneratedObject Field Access**
   - ServicePropertiesModel.properties is GeneratedObject (generic model object)
   - Need safe field extraction: `field_string(&cors, "allowedOrigins")` utility
   - May require serialization helper for header exposure extraction

**6. Constants Module** (`/home/azureuser/Azurite/rust/crates/azurite-table/src/utils/constants.rs`)
   - Verify exists: `HeaderConstants` struct with fields
   - Verify: `MethodConstants` (OPTIONS, GET, POST, etc.)
   - Verify: `ValidAPIVersions` array/set
   - Verify: `DEFAULT_TABLE_CONTEXT_PATH` constant
   - Verify: `SECONDARY_SUFFIX` constant (e.g., "-secondary")
   - Verify: `VERSION` constant

### 4.3 Integration Points

**1. Request Lifecycle Integration**
   - Middleware chain order (from generated framework):
     1. createTableStorageContextMiddleware() → extracts context
     2. createAuthenticationMiddleware() → validates credentials
     3. createOptionsHandlerMiddleware() → handles CORS preflight
     4. createCorsRequestMiddleware() → applies CORS headers
     5. createTelemetryMiddleware() → logs request
     6. Handler dispatch
   - Each middleware called in sequence with same Context object

**2. Context Thread Safety**
   - TableStorageContext wraps Context which may use Arc<Mutex<>> for shared state
   - All setters must be thread-safe (use interior mutability)
   - Logger, metadataStore passed as Arc<dyn Trait + Send + Sync>

**3. Error Handling Pattern**
   - Authentication failure: return Err(StorageError)
   - CORS failure: return Some(Box::new(StorageError))
   - Context extraction: return Err(StorageError)
   - Telemetry: panics logged but don't block processing

**4. Async Boundary**
   - All apply() methods must be async (authenticator.validate, metadataStore.getServiceProperties)
   - No blocking operations allowed
   - Error propagation: `.await?` for Result, match/if-let for Option<Error>

---

## 5. TRANSLATION SPECIFIC EDGE CASES & PITFALLS

### 5.1 TS → Rust Type Conversions
- TS `string | undefined` → Rust `Option<String>`
- TS `boolean | undefined` → Rust `Option<bool>`
- TS promise chain `.then().catch()` → Rust `.await?` or match
- TS array iteration `for()` → Rust `for ... in` or iterator methods
- TS regex test/match → Regex crate usage

### 5.2 Key Behavioral Differences

**Authentication:**
- TS: returns promise<boolean>, treats undefined as false
- Rust: returns Result<Option<bool>> where None = skip, Some(true) = pass, Some(false) = fail

**CORS Preflight:**
- TS: extracts headers from 3 sources (spec, handler, error)
- Rust: only response headers relevant (handlerResponses not available)
- May need to simplify or refactor header exposure logic

**Context Extraction:**
- TS: uses regex `/^'|'$/g` for quote stripping
- Rust: use regex crate with proper literal handling
- Escaped quote handling: `''` → `'` requires replace logic

**Error Factory Methods:**
- TS: StorageErrorFactory takes context param
- Rust: takes contextID string (extract from context.contextId())
- Additional parameters passed as Maps/objects in Rust

### 5.3 Performance Considerations
- Regex compilation: compile once (lazy_static) for URL pattern matching
- Hash sets for constant lookups: NO_ACCOUNT_HOST_NAMES should be const/lazy_static
- String allocations: minimize clones in hot path (headers iteration)

---

## SUMMARY TABLE

| TS File | Rust Target | Status | Key Complexity |
|---------|------------|--------|-----------------|
| AuthenticationMiddlewareFactory.ts | authentication_middleware_factory.rs | CREATE | Async iterator, shared Arc types, error handling |
| PreflightMiddlewareFactory.ts | preflight_middleware_factory.rs | CREATE | CORS rule matching, header serialization, wildcard logic |
| tableStorageContext.middleware.ts | table_storage_context.rs | CREATE | URL parsing state machine, quoted string extraction, secondary suffix |
| telemetry.middleware.ts | telemetry.rs | CREATE | Simple passthrough, telemetry context building |
| lib.rs (table) | middlewares/mod.rs | EDIT | Re-exports |

