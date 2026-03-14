# Queue Middlewares (Phase 14.15)

## File Info
- **Source Path:** `src/queue/middlewares/` (4 files, ~600 LOC)
- **Rust Target:** `azurite-queue/src/middlewares/`
- **Type:** Custom middleware implementations
- **Phase:** 14.15
- **Complexity:** M (medium)
- **Status:** analyzed

## Exports

```rust
pub struct AuthenticationMiddlewareFactory {
    authenticators: Vec<Box<dyn IAuthenticator>>,
}

pub struct PreflightMiddlewareFactory {
    allowed_methods: Vec<String>,
}

pub struct CORSMiddlewareFactory {
    // CORS handling
}

pub fn create_queue_storage_context_middleware(
    context: &Context,
) -> impl Middleware { ... }
```

## Detailed Exports

### AuthenticationMiddlewareFactory (~200 LOC)
**Purpose**: Validate authorization headers (SAS, Shared Key, Bearer token)

```rust
pub struct AuthenticationMiddlewareFactory {
    authenticators: Vec<Box<dyn IAuthenticator>>,
}

impl AuthenticationMiddlewareFactory {
    pub fn new(
        sas_authenticator: Box<dyn IAuthenticator>,
        shared_key_authenticator: Box<dyn IAuthenticator>,
        token_authenticator: Box<dyn IAuthenticator>,
        account_sas_authenticator: Box<dyn IAuthenticator>,
    ) -> Self { ... }

    pub async fn authenticate(
        &self,
        req: &dyn IRequest,
        context: &Context,
    ) -> Result<(), StorageError> { ... }
}
```

**Authenticator chain** (order matters):
1. Try SAS token validation
2. Try Shared Key validation
3. Try Bearer token validation
4. Try account-level SAS validation
5. If all fail, return authorization error

**Error handling**:
- Invalid signature → 403 AuthorizationPermissionMismatch
- Missing header → 403 AuthorizationHeaderMissing (context-dependent)
- Malformed header → 400 InvalidInput

### PreflightMiddlewareFactory (~150 LOC)
**Purpose**: Handle OPTIONS preflight requests (CORS)

```rust
pub struct PreflightMiddlewareFactory {
    allowed_methods: Vec<String>,
    allowed_headers: Vec<String>,
    allowed_origins: Vec<String>,
}

impl PreflightMiddlewareFactory {
    pub fn new() -> Self { ... }

    pub async fn handle_preflight(
        &self,
        req: &dyn IRequest,
        context: &Context,
    ) -> Result<PreflightResponse, StorageError> { ... }
}
```

**Response headers**:
- `Access-Control-Allow-Methods`: List of allowed HTTP methods
- `Access-Control-Allow-Headers`: List of allowed request headers
- `Access-Control-Allow-Origin`: Echo origin or wildcard
- `Access-Control-Max-Age`: Cache duration (86400 seconds = 1 day)

**Validation**:
- Origin header must be present
- Request method (from Access-Control-Request-Method) must be allowed
- Requested headers must be in allowed list

### CORSMiddlewareFactory (~150 LOC)
**Purpose**: Add CORS response headers to all responses

```rust
pub struct CORSMiddlewareFactory {
    allow_origins: Vec<String>,
}

impl CORSMiddlewareFactory {
    pub async fn add_cors_headers(
        &self,
        req: &dyn IRequest,
        resp: &mut dyn IResponse,
    ) -> Result<(), StorageError> { ... }
}
```

**Response headers added**:
- `Access-Control-Allow-Origin`: Echo origin if in allow list
- `Access-Control-Expose-Headers`: List headers exposed to client
- `Access-Control-Max-Age`: Cache control

### QueueStorageContextMiddleware (~100 LOC)
**Purpose**: Extract queue-specific context information

```rust
pub fn create_queue_storage_context_middleware(
    context: &mut Context,
) -> Result<(), ContextError> {
    // Extract queue name from request path
    // Extract message ID from path
    // Extract pop receipt from query string
    // Validate resource identifiers
    // Populate context fields for downstream handlers
    Ok(())
}
```

**Context fields populated**:
- `queueName`: Extracted from request path segment
- `messageId`: Extracted from path (for message-specific operations)
- `popReceipt`: Extracted from query parameter
- `operation`: Determined from HTTP method + path + query params
- `startTime`: Request arrival timestamp

## Dependencies

- Phase 14.1: Context, IRequest, IResponse
- Phase 14.6: IAuthenticator implementations
- Phase 14.2-3: StorageErrorFactory
- Common: HeaderConstants, utility functions

## Type Mappings

| TypeScript | Rust Equivalent | Notes |
|-----------|-----------------|-------|
| `Middleware` function | `impl Fn(req, resp, next) -> Future` | Middleware trait object |
| `async function` | `async fn` | Middleware handlers |
| `Promise<void>` | `Result<(), Error>` | Async operations |
| `IRequest` | `&dyn IRequest` | Request reference |
| `IResponse` | `&mut dyn IResponse` | Mutable response reference |
| `string[]` | `Vec<String>` | Headers, methods, origins |
| `string` | `String` | Header values, origins |
| `RegExp` | `Regex` | Origin pattern matching |

## Special Handling / Fidelity Flags

### Critical Middleware Order
The queue service uses this middleware order:
1. **Access log** — Log all requests
2. **Queue context creation** — Extract queue-specific context
3. **Dispatch** — Route to correct handler
4. **Strict mode** — Validate strict operation semantics
5. **Authentication** — Validate authorization
6. **Deserializer** — Parse request body
7. **Handler invocation** — Call specific handler
8. **CORS (error path)** — Add CORS headers on error
9. **CORS (success path)** — Add CORS headers on success
10. **Serializer** — Format response
11. **OPTIONS preflight** — Handle CORS preflight
12. **Error handler** — Handle exceptions
13. **Telemetry** — Record metrics
14. **End** — Finalize response

**Critical**: Reordering these breaks request processing. For example:
- Authentication MUST come after context creation but before handler
- CORS must apply to both error and success paths
- Serializer must come before end middleware

### Authentication Chain Semantics
1. Authenticators are tried in order
2. First one that returns `Ok(Some(true))` stops the chain
3. Returns `Ok(None)` → try next authenticator
4. Returns `Ok(Some(false))` → authentication failed; may fallback to anonymous (depends on policy)
5. Returns `Err(_)` → hard error; fail immediately

### CORS vs Preflight
- **Preflight (OPTIONS)**: Separate request handler; responds with allowed methods/headers
- **CORS response headers**: Applied to all responses (GET, POST, etc.)
- **Both use same origin validation logic**: Allow-listed origins only

### Comparison to Blob Middlewares
- **Identical structure**: Same middleware order and naming
- **Queue context**: Simpler than blob (no container/blob/snapshot complexity)
- **Authenticators**: Same SAS/Shared Key/Bearer pattern
- **CORS**: Identical implementation

## Change Propagation

**Authenticator list changes:**
- Adding/removing authenticator requires middleware factory update
- Order matters; inserting in wrong position breaks auth chain

**CORS allow-list changes:**
- Dynamic vs static allow-lists affect middleware initialization
- If CORS rules stored in service properties, requires loading from metadata store

**Context field additions:**
- New queue-specific fields may require path parsing changes
- Downstream handlers may expect new context fields

## Rust Porting Notes

1. **Middleware trait**: Implement as `Box<dyn Middleware>` or `impl Fn() -> Future`
2. **Authenticator chain**: Use `for` loop with early return on success
3. **CORS header matching**: Use lowercase for header name comparison
4. **Origin validation**: Use glob/regex for pattern matching if needed
5. **Async context**: All middleware is async; use `tokio`
6. **Error propagation**: Convert auth errors to appropriate HTTP status codes
7. **Path parsing**: Use regex or string parsing for queue name/message ID extraction
8. **Query string parsing**: Use `url` crate for parameter extraction
9. **Time measurement**: `std::time::Instant` for request timing
10. **Header mutations**: Clone headers if needed; some middleware may need to modify them

## Implementation Strategy

Propose 2-file porting units for Phase 14.15:
1. **Unit 1**: AuthenticationMiddlewareFactory + QueueStorageContextMiddleware (context + auth setup)
2. **Unit 2**: PreflightMiddlewareFactory + CORSMiddlewareFactory (CORS handling)
