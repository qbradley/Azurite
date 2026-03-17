# Table Service Middleware Translation - Quick Reference

## File Creation Checklist

### 1. `/home/azureuser/Azurite/rust/crates/azurite-table/src/middlewares/mod.rs`
**Action:** EDIT (currently empty)

```rust
pub mod authentication_middleware_factory;
pub mod preflight_middleware_factory;
pub mod table_storage_context;
pub mod telemetry;

pub use authentication_middleware_factory::{
    AuthenticationMiddleware, AuthenticationMiddlewareFactory, 
    SharedAuthenticator, SharedMiddlewareLogger,
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

### 2. `authentication_middleware_factory.rs` (CREATE)

**Summary:** Sequential validation of multiple `IAuthenticator` instances. Returns `Ok(())` on first success or `Err(AuthorizationFailure)` if all fail.

**Key Methods:**
```rust
pub struct AuthenticationMiddleware {
    logger: Arc<dyn ILogger + Send + Sync>,
    authenticators: Arc<Vec<Arc<dyn IAuthenticator + Send + Sync>>>,
}

impl AuthenticationMiddleware {
    pub async fn apply(
        &self,
        context: &TableStorageContext,
        req: &GeneratedHttpRequest,
        res: &GeneratedHttpResponse,
    ) -> Result<(), StorageError>
}
```

**Logic:**
```rust
for authenticator in self.authenticators.iter() {
    if let Some(pass) = authenticator.validate(req, context).await? {
        return Ok(());
    }
}
Err(StorageErrorFactory::getAuthorizationFailure(context.contextId()))
```

---

### 3. `preflight_middleware_factory.rs` (CREATE)

**Summary:** CORS preflight handling. Two middleware types: OptionsHandlerMiddleware for OPTIONS requests, CorsRequestMiddleware for normal requests.

**Key Structures:**
```rust
#[derive(Clone)]
pub struct OptionsHandlerMiddleware {
    factory: PreflightMiddlewareFactory,
    metadataStore: Arc<dyn ITableMetadataStore + Send + Sync>,
}

pub async fn apply(&self, err: BoxedMiddlewareError, ...) -> Option<BoxedMiddlewareError>
// If OPTIONS: validate Origin/Access-Control-Request-Method, match CORS rules, set headers
// If matched: return None (continue), else: return Some(corsPreflightFailure error)

#[derive(Clone)]
pub struct CorsRequestMiddleware { ... }
pub async fn apply(&self, err: Option<BoxedMiddlewareError>, ...) -> Option<BoxedMiddlewareError>
// If OPTIONS: pass through error as-is
// If normal request: apply CORS headers from matched rules
```

**Helper Functions:**
```rust
fn checkOrigin(origin: &str, allowedOrigins: &str) -> bool
// allowedOrigins: comma-sep list, supports "*"
// Case-insensitive comparison

fn checkMethod(method: &str, allowedMethods: &str) -> bool
// allowedMethods: comma-sep list, supports "*"
// Case-insensitive

fn checkHeaders(headers: &str, allowedHeaders: &str) -> bool
// headers: comma-sep list from request
// allowedHeaders: comma-sep list, supports "prefix*" wildcard
// All headers must match some rule
```

---

### 4. `table_storage_context.rs` (CREATE)

**Summary:** Extracts storage context from URL: account, tableName, partitionKey, rowKey. Complex URL parsing with state machine.

**Key Functions:**
```rust
pub fn createTableStorageContextMiddleware(
    skipApiVersionCheck: Option<bool>,
    disableProductStyleUrl: Option<bool>,
) -> TableStorageContextMiddlewareOptions

pub fn tableStorageContextMiddleware(
    context: &Context,
    req: &GeneratedHttpRequest,
    res: &mut GeneratedHttpResponse,
    logger: &(dyn ILogger + Send + Sync),
    options: TableStorageContextMiddlewareOptions,
) -> Result<(), StorageError>

pub fn extractStoragePartsFromPath(
    hostname: &str,
    path: &str,
    disableProductStyleUrl: bool,
) -> (Option<String>, Option<String>, bool)  // (account, tableSection, isSecondary)
```

**URL Parsing Logic:**
```
Path "/account/Tables" → account="account", tableSection="Tables"
Path "/account/mytable(PartitionKey='pk',RowKey='rk')" → 
  → account="account", tableSection="mytable(PartitionKey='PLACEHOLDER',RowKey='PLACEHOLDER')"
  → Extract partitionKey="pk", rowKey="rk"

Hostname "myaccount.localhost:10002" (production-style) →
  → account="myaccount" (from subdomain)

Account "myaccount-secondary" → remove suffix → account="myaccount", isSecondary=true
```

**Context Fields Set:**
- `account`, `tableName`, `partitionKey`, `rowKey`
- `authenticationPath` (path with secondary suffix removed)
- `dispatchPattern` (tableSection for dispatch)
- `isSecondary` (boolean)
- `accept` (from header)
- `xMsRequestID` (UUID)

**Validations:**
- API version check (unless skipApiVersionCheck=true)
- Table name validation (except system tables starting with $)

---

### 5. `telemetry.rs` (CREATE)

**Summary:** Simple telemetry collection. Factory creates middleware that calls `AzuriteTelemetryClient::TraceRequest()`.

**Key Code:**
```rust
pub fn telemetryMiddleware(context: &Context) {
    AzuriteTelemetryClient::TraceRequest(build_telemetry_context(context));
}

#[derive(Clone)]
pub struct TelemetryMiddleware {
    contextPath: String,
}

impl TelemetryMiddleware {
    pub fn apply(&self, context: &Context) {
        telemetryMiddleware(context);
    }
}

#[derive(Clone)]
pub struct TelemetryMiddlewareFactory { ... }
impl TelemetryMiddlewareFactory {
    pub fn createTelemetryMiddleware(&self) -> TelemetryMiddleware { ... }
}
```

---

## Type Aliases (in each module's pub use)

```rust
pub type SharedAuthenticator = Arc<dyn IAuthenticator + Send + Sync>;
pub type SharedMiddlewareLogger = Arc<dyn ILogger + Send + Sync>;
pub type SharedTableMetadataStore = Arc<dyn ITableMetadataStore + Send + Sync>;
pub type BoxedMiddlewareError = Box<dyn std::error::Error + Send + Sync>;
```

---

## Critical Dependencies to Verify

| Dependency | Location | Status |
|-----------|----------|--------|
| `TableStorageContext` | `azurite-table/context/table_storage_context.rs` | ✓ Exists |
| `IAuthenticator` trait | `azurite-table/authentication/i_authenticator.rs` | ✓ Exists |
| `ITableMetadataStore` trait | `azurite-table/persistence/i_table_metadata_store.rs` | ✓ Exists |
| `StorageErrorFactory` | `azurite-table/errors/storage_error_factory.rs` | VERIFY |
| `ILogger` trait | `azurite-common/i_logger.rs` | ✓ Exists |
| `HeaderConstants` | `azurite-table/utils/constants.rs` | VERIFY |
| `MethodConstants` | `azurite-table/utils/constants.rs` | VERIFY |
| `ValidAPIVersions` | `azurite-table/utils/constants.rs` | VERIFY |
| `SECONDARY_SUFFIX` | `azurite-table/utils/constants.rs` | VERIFY |
| `VERSION` | `azurite-table/utils/constants.rs` | VERIFY |
| `IP_REGEX` | `azurite-common/utils/constants.rs` | ✓ Exists |
| `NO_ACCOUNT_HOST_NAMES` | `azurite-common/utils/constants.rs` | ✓ Exists |
| `GeneratedHttpRequest` | `azurite-table/generated/i_request.rs` | ✓ Exists |
| `GeneratedHttpResponse` | `azurite-table/generated/i_response.rs` | ✓ Exists |
| `Context` | `azurite-table/generated/context.rs` | ✓ Exists |
| `ResponseHeaderValue` | `azurite-table/generated/i_response.rs` | ✓ Exists |

---

## Implementation Order

1. **Verify constants and types** exist in `azurite-table/utils/constants.rs`
2. **Implement `telemetry.rs`** (simplest, minimal dependencies)
3. **Implement `authentication_middleware_factory.rs`** (simple logic, validates IAuthenticator interface)
4. **Implement `table_storage_context.rs`** (complex, but isolated)
5. **Implement `preflight_middleware_factory.rs`** (most complex, depends on error factory methods)
6. **Edit `mod.rs`** (final re-exports)
7. **Integration testing** with middleware chain

---

## Key Edge Cases to Watch

| Edge Case | TS Behavior | Rust Implementation |
|-----------|------------|-------------------|
| Multiple CORS rules | **First match** wins | Iterate, return on first match |
| Escaped quotes in keys | `''` → `'` in extracted values | Regex + replace |
| Secondary suffix | Remove from account AND authPath | String trim on both fields |
| Missing Origin header | Return 400 invalid CORS header | return Err(getInvalidCorsHeaderValue) |
| No CORS rules configured | Return 403 corsPreflightFailure | return Err(corsPreflightFailure) |
| GET with entity key | Convert tableSection from `table(...)` to `table()` | String replacement logic |
| System table | Skip validation (name starts with $) | Check tableName.starts_with("$") |
| IP address hostname | Use path-based account | IP_REGEX.is_match(hostname) |
| Wildcard in headers | Match prefix: "x-ms-meta-*" | str.ends_with("*") && str.trim_end_matches('*') |
| Empty authenticators | All requests pass? | Depends on TS behavior - clarify |

---

## Testing Considerations

1. **URL parsing**: Test all 6 table section variants
2. **CORS matching**: Test wildcard origins, methods, headers
3. **Secondary suffix**: Test account-secondary removal
4. **Quoted string extraction**: Test escaped quotes `''`
5. **Authentication**: Test with 0, 1, N authenticators
6. **Error cases**: Missing headers, invalid CORS, auth failure

---

## Performance Notes

- Compile regexes once using `lazy_static` or `once_cell`:
  - `IP_REGEX` for hostname validation
  - Quoted string extraction regex: `/'([^']|'')*'/g`
  
- Use string interning for constant header names (HeaderConstants)

- Avoid string clones in hot path (header iteration)

---

## Reference Files

- TS Source: `/home/azureuser/Azurite/src/table/middleware/`
- Rust Blob Example: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/middlewares/`
- Target: `/home/azureuser/Azurite/rust/crates/azurite-table/src/middlewares/`
- Full Report: `/home/azureuser/TABLE_MIDDLEWARE_TRANSLATION.md`

