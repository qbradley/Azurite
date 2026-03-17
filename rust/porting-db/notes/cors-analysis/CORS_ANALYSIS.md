# Azurite CORS Implementation: TypeScript vs Rust Analysis

## Executive Summary
The TypeScript implementation has a working CORS system, while the Rust implementation has fundamental issues with middleware ordering and error handling. The main problems are:

1. **Middleware ordering is wrong**: `corsErrorRequestMiddleware` and `corsRequestMiddleware` are applied AFTER `serializer_middleware`, but they should be BEFORE
2. **OPTIONS handling is broken**: `optionsHandlerMiddleware` executes AFTER `error_middleware`, when an OPTIONS request hasn't matched any route
3. **Missing error handling chain**: When OPTIONS request has no matching CORS rules, it passes through serialization and only then hits optionsHandlerMiddleware as an error

---

## PART 1: TypeScript CORS (Reference Implementation)

### 1.1 Core CORS Middleware File
**File**: `/home/azureuser/Azurite/src/blob/middlewares/PreflightMiddlewareFactory.ts` (466 lines)

Key classes:
- `PreflightMiddlewareFactory` - main factory
- Two methods:
  - `createOptionsHandlerMiddleware()` - error handler for OPTIONS requests
  - `createCorsRequestMiddleware()` - adds CORS headers to normal requests

### 1.2 OPTIONS Preflight Request Handling

**Method**: `createOptionsHandlerMiddleware()` (lines 28-141)
- **Type**: ErrorRequestHandler (Express error middleware)
- **Triggered**: Only if `req.method === "OPTIONS"`
- **Logic Flow**:
  ```typescript
  1. Extract Origin header (REQUIRED - fails if missing/invalid)
  2. Extract Access-Control-Request-Method header (REQUIRED)
  3. Extract Access-Control-Request-Headers (optional)
  4. Call metadataStore.getServiceProperties(context, account)
  5. For each CORS rule:
     - checkOrigin() - wildcard match against allowedOrigins
     - checkMethod() - exact match against allowedMethods
     - checkHeaders() - exact/wildcard match against allowedHeaders
  6. If first rule matches ALL checks:
     - Set statusCode (implicit 200)
     - Set Access-Control-Allow-Origin
     - Set Access-Control-Allow-Methods
     - Set Access-Control-Allow-Headers (if requested)
     - Set Access-Control-Max-Age
     - Set Access-Control-Allow-Credentials: "true"
     - Call next() with NO error
  7. If NO rule matches:
     - Return error via next(StorageErrorFactory.corsPreflightFailure())
  ```

**Critical Detail**: This is an ErrorRequestHandler, so it receives `err` as first parameter. On line 37, it checks if request is OPTIONS. On line 138, if not OPTIONS, it passes the error through: `next(err)`.

### 1.3 CORS Headers on Normal (Non-OPTIONS) Requests

**Method**: `createCorsRequestMiddleware()` (lines 143-230)
- **Type**: Can return ErrorRequestHandler or RequestHandler based on `blockErrorRequest` parameter
- **Logic Flow**:
  ```typescript
  1. Skip if req.method === "OPTIONS" (line 153-154)
  2. Check if Origin header exists
  3. Call metadataStore.getServiceProperties(context, account)
  4. For each CORS rule:
     - checkOrigin() && checkMethod()
     - If match found:
       * getExposedHeaders() - collect response headers matching exposedHeaders rules
       * Set Access-Control-Expose-Headers
       * Set Access-Control-Allow-Origin:
         - Use "*" if allowedOrigins === "*"
         - Otherwise use actual origin (line 201)
       * If NOT wildcard origin:
         - Set Vary: "Origin"
         - Set Access-Control-Allow-Credentials: "true"
       * Return next(err) with original error
  5. If no CORS rules match but rules exist:
     - Set Vary: "Origin" (line 216)
  6. Return next(err) - always continues
  ```

**Key Insight**: This middleware ALWAYS calls `next()`, even with errors. CORS headers are optional - the request continues regardless.

### 1.4 Wildcard Origin Matching

**Method**: `checkOrigin()` (lines 232-257)
```typescript
if (allowedOrigin === "*") return true;  // Wildcard matches anything

const allowedOriginArray = allowedOrigin.split(",");
for (const corsOrigin of allowedOriginArray) {
  if (corsOrigin.includes("*")) {
    // Use glob-to-regexp to convert wildcard pattern to regex
    return glob(corsOrigin.trim().toLowerCase()).test(origin.trim().toLowerCase());
  }
  if (origin.trim().toLowerCase() === corsOrigin.trim().toLowerCase()) {
    return true;
  }
}
```

**Implementation**: Uses `glob-to-regexp` npm package
- `*.contoso.com` → regex that matches `foo.contoso.com`
- Case-insensitive matching
- Multiple origins supported (comma-separated)

### 1.5 Service Properties Storage

**File**: `/home/azureuser/Azurite/src/blob/handlers/ServiceHandler.ts`
- **setProperties()** (lines 166-204):
  - Receives `storageServiceProperties: Models.StorageServiceProperties`
  - Normalizes: sets `allowedHeaders` and `exposedHeaders` to empty string if undefined
  - Calls `metadataStore.setServiceProperties(context, {...storageServiceProperties, accountName})`
  - Returns 202 status

- **Default Properties** (lines 57-58):
  - `cors: []` - empty array by default

- **getProperties()** (lines 206-236):
  - Calls `metadataStore.getServiceProperties(context, accountName)`
  - Returns `properties.cors` array to client

### 1.6 Middleware Pipeline Order (BlobRequestListenerFactory.ts lines 135-223)

```
1. createDispatchMiddleware() - route the operation
2. createStrictModelMiddleware() - validate headers/params
3. createAuthenticationMiddleware() - authenticate
4. createDeserializerMiddleware() - parse body/params
5. createHandlerMiddleware() - execute business logic
6. *** corsErrorRequestMiddleware(true) *** - add CORS on errors
7. *** corsRequestMiddleware(false) *** - add CORS on success
8. createSerializerMiddleware() - serialize response
9. *** createOptionsHandlerMiddleware() *** - handle OPTIONS that weren't already matched
10. createErrorMiddleware() - format errors
11. createEndMiddleware() - finalize response
```

**Key Points**:
- CORS middleware runs BEFORE serialization (lines 192-203)
- Specifically: corsErrorRequestMiddleware(true) captures errors from handlers
- Then corsRequestMiddleware(false) runs on success path
- OPTIONS handler is LAST ERROR handler (line 209-212)

---

## PART 2: Rust CORS Implementation

### 2.1 Core CORS Middleware File
**File**: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/middlewares/preflight_middleware_factory.rs` (512 lines)

Key structs:
- `PreflightMiddlewareFactory` - factory
- `OptionsHandlerMiddleware` - OPTIONS handler wrapper
- `CorsRequestMiddleware` - CORS header wrapper

### 2.2 OPTIONS Preflight Request Handling

**Method**: `apply_options()` (lines 109-220)
- **Input**: `err: BoxedMiddlewareError` (always has an error from previous middleware)
- **Logic**:
  ```rust
  1. If method != OPTIONS: return Some(err)  // Pass error through
  2. Extract origin (required)
  3. Extract requestMethod/Access-Control-Request-Method (required)
  4. Extract requestHeaders (optional)
  5. Call metadataStore.getServiceProperties(context, &account)
  6. Set response status code to 200
  7. For each CORS rule in cors_rules():
     - checkOrigin() && checkMethod()
     - If headers present: checkHeaders()
     - If all match:
       * Set CORS response headers
       * Return None (no error)
  8. If no match:
     - Return Some(corsPreflightFailure error)
  ```

### 2.3 CORS Headers on Normal Requests

**Method**: `apply_cors_request()` (lines 222-302)
- **Input**: `err: Option<BoxedMiddlewareError>`
- **Logic**:
  ```rust
  1. If method == OPTIONS: return err  // Don't process
  2. If blockErrorRequest && err.is_none(): return None  // BROKEN: drops error!
  3. Check for Origin header (optional - no error if missing)
  4. For each CORS rule:
     - checkOrigin() && checkMethod()
     - If match:
       * Set Access-Control-Expose-Headers
       * Set Access-Control-Allow-Origin
       * If not wildcard: set Vary and Credentials
       * Return err (pass through)
  5. If rules exist but no match:
     - Set Vary: "Origin"
  6. Return err
  ```

**CRITICAL BUG on lines 235-237**:
```rust
if blockErrorRequest && err.is_none() {
    return None;  // DROPS THE ERROR!
}
```
When `blockErrorRequest=true` (first call) AND no error exists, it returns `None`, dropping any pending error. This breaks OPTIONS error handling!

### 2.4 Wildcard Origin Matching

**Function**: `wildcard_regex()` (lines 500-507)
```rust
fn wildcard_regex(pattern: &str) -> Option<Regex> {
    let mut escaped = regex::escape(pattern);
    escaped = escaped.replace("\\*", ".*");
    regex::RegexBuilder::new(&format!("^{escaped}$"))
        .case_insensitive(true)
        .build()
        .ok()
}
```

**Implementation**: 
- Escape regex special chars
- Replace `\*` (escaped wildcard) with `.*`
- Build case-insensitive regex
- Uses `regex` crate (not glob)

### 2.5 Service Properties Storage & Retrieval

**File**: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/persistence/i_blob_metadata_store.rs`

**Struct**: `ServicePropertiesModel` (lines 40-43)
```rust
pub struct ServicePropertiesModel {
    pub accountName: String,
    pub properties: StorageServiceProperties,  // Contains the actual CORS array
}
```

**Interface** (in i_blob_metadata_store.rs):
```rust
async fn setServiceProperties(
    &self,
    context: &Context,
    serviceProperties: ServicePropertiesModel,
) -> Result<ServicePropertiesModel, StorageError>;

async fn getServiceProperties(
    &self,
    context: &Context,
    account: &str,
) -> Result<Option<ServicePropertiesModel>, StorageError>;
```

**Usage in handler** (service_handler.rs lines 145-154):
```rust
self.base.metadataStore.setServiceProperties(
    &context,
    ServicePropertiesModel {
        accountName,
        properties: storageServiceProperties,
    },
).await?;
```

### 2.6 Middleware Pipeline Order (blob_request_listener_factory.rs)

**From RequestListenerState::handle_request()** (lines 140-279):

```
1. internalBlobStorageContextMiddleware() - set up context
2. dispatch_middleware() - route operation
3. strictModelMiddleware.apply() - validate if present
4. authenticationMiddleware.apply() - authenticate
5. deserializer_middleware() - parse request
6. HandlerMiddlewareFactory.call() - execute handler
7. *** corsErrorRequestMiddleware.apply(Some(error), ...) *** - process ERROR path
8. *** corsRequestMiddleware.apply(None, ...) *** - process SUCCESS path (if no error)
9. serializer_middleware() - serialize response
10. *** optionsHandlerMiddleware.apply(error, ...) *** - handle OPTIONS
11. error_middleware() - format errors
12. end_middleware() - finalize response
```

**Lines 222-244 (corsErrorRequestMiddleware)**:
```rust
if let Some(error) = pending_error.take() {
    pending_error = self.corsErrorRequestMiddleware
        .apply(Some(error), &blob_context, &generated_request, &mut generated_response)
        .await;
}

if pending_error.is_none() {  // Only if error was resolved
    pending_error = self.corsRequestMiddleware
        .apply(None, &blob_context, &generated_request, &mut generated_response)
        .await;
}
```

**Lines 246-264 (optionsHandlerMiddleware)**:
```rust
if pending_error.is_none() {
    if let Err(error) = serializer_middleware(...) {
        pending_error = Some(error);
    }
}

if let Some(error) = pending_error.take() {
    pending_error = self.optionsHandlerMiddleware
        .apply(error, &blob_context, &generated_request, &mut generated_response)
        .await;
}
```

### 2.7 Helper Functions

**cors_rules()** (lines 479-494):
```rust
fn cors_rules(properties: &ServicePropertiesModel) -> Vec<GeneratedObject> {
    properties.properties
        .get("cors")
        .or_else(|| properties.properties.get("Cors"))  // Support both cases
        .and_then(|value| match value {
            GeneratedValue::Array(values) => Some(...),
            _ => None,
        })
        .unwrap_or_default()  // Return empty vec if not found
}
```

Extracts CORS array from the dynamic GeneratedObject properties map.

**field_string()** (lines 496-498):
```rust
fn field_string(value: &GeneratedObject, key: &str) -> Option<String> {
    value.get(key).and_then(GeneratedValue::as_string)
}
```

Safe extraction of string fields from CORS rules.

---

## PART 3: Key Differences & Issues

### 3.1 Middleware Ordering Problem (CRITICAL)

| Step | TypeScript | Rust |
|------|-----------|------|
| Handler execution | ✓ | ✓ |
| Error captured | ✓ | ✓ |
| **CORS processing** | **BEFORE serialization** | **AFTER serialization** |
| Serializer runs | ✓ | ✓ |
| OPTIONS handling | ✓ (after serializer) | ✗ (after error_middleware) |
| Error formatting | ✓ | ✓ |

**Rust Problem**: Line 246 serializer runs BEFORE line 256 optionsHandlerMiddleware. By then:
- Response is already serialized
- Error has already been formatted
- OPTIONS handler can't properly intercept

### 3.2 CORS Middleware blockErrorRequest Bug (CRITICAL)

**TypeScript** (BlobRequestListenerFactory.ts lines 192-203):
```typescript
// First call with blockErrorRequest=true
app.use(preflightMiddlewareFactory.createCorsRequestMiddleware(
    this.metadataStore,
    true
));
// Second call with blockErrorRequest=false
app.use(preflightMiddlewareFactory.createCorsRequestMiddleware(
    this.metadataStore,
    false
));
```

Both calls forward the error. The middleware handles it:
```typescript
const internalMethod = (err: MiddlewareError | Error | undefined, ...) => {
  if (req.method.toUpperCase() === MethodConstants.OPTIONS) {
    return next(err);  // Pass through OPTIONS to next middleware
  }
  // ... CORS logic for other methods
}
```

**Rust** (blob_request_listener_factory.rs lines 234-244):
```rust
// corsErrorRequestMiddleware with blockErrorRequest=true
if let Some(error) = pending_error.take() {
    pending_error = self.corsErrorRequestMiddleware
        .apply(Some(error), ...)
        .await;
}

// corsRequestMiddleware with blockErrorRequest=false
if pending_error.is_none() {  // Only if NO error!
    pending_error = self.corsRequestMiddleware
        .apply(None, ...)
        .await;
}
```

**And in apply_cors_request()** (lines 235-237):
```rust
if blockErrorRequest && err.is_none() {
    return None;  // BUG: Drops error for successful requests!
}
```

This causes:
1. First CORS middleware (blockErrorRequest=true): If error exists, tries to recover but
2. Line 235-237 bug: If no error yet exists, returns None (consumes it)
3. Second CORS middleware never runs because error was dropped

### 3.3 Origin Matching Algorithm Differences

**TypeScript**: 
- Uses `glob-to-regexp` npm package
- Pattern: `*.contoso.com`
- Converts glob syntax to regex
- Example: `*.foo.com` matches `a.foo.com`, `b.foo.com`, etc.

**Rust**:
- Uses `regex` crate with manual glob conversion
- Pattern: `*.contoso.com`
- Manual escape + replace logic
- Should be functionally equivalent

**Potential issue**: Regex might handle some edge cases differently than glob.

### 3.4 Error Handling in OPTIONS

**TypeScript** (PreflightMiddlewareFactory.ts lines 28-141):
- ErrorRequestHandler - receives err as first param
- Checks method first (line 37)
- If OPTIONS: validates headers and CORS rules
- Returns error via `next()` only if validation fails
- Properly chains errors

**Rust** (preflight_middleware_factory.rs lines 109-220):
- Takes `err: BoxedMiddlewareError` - error is REQUIRED
- If method != OPTIONS: returns `Some(err)` immediately
- If method == OPTIONS: tries to handle but
- Options are checked AFTER serialization has already run
- Error metadata might be lost by then

### 3.5 Vary Header Handling

**TypeScript** (lines 204-217):
```typescript
if (cors.allowedOrigins !== "*") {
    res.setHeader(HeaderConstants.VARY, "Origin");
    res.setHeader(HeaderConstants.ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
}

// ... later, if no CORS rules match:
if (corsSet.length > 0) {
    res.setHeader(HeaderConstants.VARY, "Origin");
}
```

**Rust** (lines 281-299):
```rust
if allowedOrigins.trim() != "*" {
    res.setHeader(HeaderConstants::VARY, ...);
    res.setHeader(HeaderConstants::ACCESS_CONTROL_ALLOW_CREDENTIALS, ...);
}

// ... later:
if !corsSet.is_empty() {
    res.setHeader(HeaderConstants::VARY, ...);
}
```

Both implementations match - Vary header set when origin-dependent CORS rules exist.

---

## PART 4: Root Causes of Rust CORS Failures

### Problem 1: OPTIONS Requests Never Reach Handler

**What happens in Rust**:
1. OPTIONS request arrives
2. dispatch_middleware() doesn't have a route for OPTIONS (it's not explicitly defined)
3. Returns error (404 or similar)
4. Error flows through CORS middleware
5. corsErrorRequestMiddleware.apply_cors_request() skips OPTIONS (line 231)
6. corsRequestMiddleware.apply_cors_request() skips OPTIONS (line 231)
7. serializer_middleware() tries to serialize error (won't include CORS headers)
8. optionsHandlerMiddleware.apply_options() finally gets it
   - But response is already serialized!
   - Error formatter has already run!
   - Can't set proper CORS headers now

**Compare to TypeScript**:
1. Express has no route for OPTIONS
2. Error is "method not implemented" or similar
3. Flows through CORS middleware before serialization
4. corsRequestMiddleware adds CORS headers to error response
5. optionsHandlerMiddleware runs and can override response completely
6. Handler runs last and formats error

### Problem 2: The blockErrorRequest Parameter Bug

**In apply_cors_request()** (lines 235-237):
```rust
if blockErrorRequest && err.is_none() {
    return None;  // Returns None instead of err!
}
```

Should be:
```rust
if blockErrorRequest && err.is_some() {
    return None;  // Only drop error if blockErrorRequest AND error exists
}
```

Current logic:
- blockErrorRequest=true (first call): If NO error exists, returns None
  - This is wrong - should pass through
- blockErrorRequest=false (second call): Never called if first one returned None

### Problem 3: Missing OPTIONS Route Registration

**TypeScript** (BlobRequestListenerFactory.ts): No explicit route registration - Express handles OPTIONS automatically

**Rust** (blob_request_listener_factory.rs line 454):
```rust
Router::new()
    .route("/", any(blob_request_listener))
    .route("/{*path}", any(blob_request_listener))
    .with_state(state)
```

Uses `any()` which should accept all methods, but the internal dispatch middleware needs to properly handle OPTIONS.

---

## PART 5: Required Fixes

### Fix 1: Reorder Middleware Pipeline

Move CORS middleware BEFORE serialization:

**Current order**:
```
Handler → corsErrorMiddleware → corsRequestMiddleware → Serializer → OptionsHandler → ErrorFormatter
```

**Required order**:
```
Handler → corsErrorMiddleware → corsRequestMiddleware → OptionsHandler → Serializer → ErrorFormatter
```

### Fix 2: Fix blockErrorRequest Bug

In `apply_cors_request()` line 235-237, change logic:
```rust
// Don't drop error for successful requests
// blockErrorRequest only matters if there's an error to block
if blockErrorRequest && err.is_some() {
    // Only return None if we're intentionally blocking this error
    // Currently: never blocks anything
}
```

Or re-evaluate the entire blockErrorRequest logic against TypeScript behavior.

### Fix 3: Ensure OPTIONS Requests Properly Dispatch

The dispatch_middleware() needs to allow OPTIONS through (even if 404), or optionsHandlerMiddleware needs to run before dispatch failures are considered final.

### Fix 4: Set Correct Status Code for OPTIONS

**TypeScript** (line 189): Implicit 200 status in successful OPTIONS preflight

**Rust** (line 188): Explicitly sets `res.setStatusCode(200)`

Verify this doesn't conflict with earlier error handling.

