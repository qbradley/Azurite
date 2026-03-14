# Queue Server, GC, Utils (Phase 14.16-14.24)

## File Info
- **Source Path:** `src/queue/` (9 files, ~820 LOC)
- **Rust Target:** `azurite-queue/src/`
- **Type:** Server bootstrap, GC, utilities
- **Phase:** 14.16-14.24
- **Complexity:** M (medium)
- **Status:** analyzed

## Exports

### IQueueEnvironment & QueueEnvironment (14.16-14.17)
```rust
// IQueueEnvironment.ts (30 LOC)
pub trait IQueueEnvironment: IEnvironment {
    fn queue_database_path(&self) -> Result<String, EnvironmentError>;
    fn queue_extent_store_path(&self) -> Result<String, EnvironmentError>;
    fn queue_keep_alive_timeout(&self) -> Result<i32, EnvironmentError>;
    fn queue_gc_interval(&self) -> Result<i32, EnvironmentError>;
    fn queue_gc_max_items(&self) -> Result<i32, EnvironmentError>;
    fn queue_gc_keep_secs(&self) -> Result<i32, EnvironmentError>;
}

// QueueEnvironment.ts (170 LOC)
pub struct QueueEnvironment { ... }

impl QueueEnvironment {
    pub fn new(argv: Vec<String>) -> Result<Self, EnvironmentError> { ... }
}

impl IQueueEnvironment for QueueEnvironment {
    fn queue_database_path(&self) -> Result<String, EnvironmentError> { ... }
    fn queue_extent_store_path(&self) -> Result<String, EnvironmentError> { ... }
    fn queue_keep_alive_timeout(&self) -> Result<i32, EnvironmentError> { ... }
    fn queue_gc_interval(&self) -> Result<i32, EnvironmentError> { ... }
    fn queue_gc_max_items(&self) -> Result<i32, EnvironmentError> { ... }
    fn queue_gc_keep_secs(&self) -> Result<i32, EnvironmentError> { ... }
}
```

**Environment variables** (with defaults):
- `AZURITE_QUEUE_DATABASE_PATH`: Path to Loki database (default: `./db`)
- `AZURITE_QUEUE_EXTENT_STORE_PATH`: Path to extent store (default: `./extents`)
- `AZURITE_QUEUE_KEEP_ALIVE_TIMEOUT`: HTTP keep-alive timeout (default: 30000 ms)
- `AZURITE_QUEUE_GC_INTERVAL`: GC check interval (default: 1800000 ms)
- `AZURITE_QUEUE_GC_MAX_ITEMS`: GC batch size (default: 1000)
- `AZURITE_QUEUE_KEEP_SECS`: Keep extents for X seconds (default: 3600 s)

**Lazy validation**: Getters validate on access (no constructor validation)

### QueueConfiguration (14.18)
```rust
// QueueConfiguration.ts (66 LOC)
pub struct QueueConfiguration extends ConfigurationBase {
    pub fn new(argv: Vec<String>) -> Result<Self, ConfigurationError> { ... }
}

impl ConfigurationBase for QueueConfiguration {
    fn validate_configuration(&self) -> Result<(), ConfigurationError> { ... }
}
```

**Validation logic**:
- Mutually exclusive: `inMemoryPersistence` OR `location` (one must be set)
- Both set → error
- Neither set → error

### QueueRequestListenerFactory (14.19)
```rust
pub struct QueueRequestListenerFactory {
    metadata_store: Arc<dyn IQueueMetadataStore>,
    extent_store: Arc<dyn IExtentStore>,
    logger: Arc<dyn ILogger>,
}

impl IRequestListenerFactory for QueueRequestListenerFactory {
    fn create_request_listener(&self) -> Box<dyn RequestListener> { ... }
}
```

**Middleware assembly order**:
1. Access log middleware
2. Queue storage context middleware
3. Dispatch middleware
4. Strict mode middleware
5. Authentication middleware
6. Deserializer middleware
7. Handler middleware
8. CORS middleware (error path)
9. CORS middleware (success path)
10. Serializer middleware
11. OPTIONS preflight middleware
12. Error middleware
13. Telemetry middleware
14. End middleware

### QueueServer (14.20)
```rust
pub struct QueueServer extends ServerBase {
    configuration: Arc<QueueConfiguration>,
    environment: Arc<dyn IQueueEnvironment>,
    metadata_store: Arc<dyn IQueueMetadataStore>,
    extent_store: Arc<dyn IExtentStore>,
}

impl QueueServer {
    pub fn new(configuration: Arc<QueueConfiguration>) -> Result<Self, ServerError> { ... }
    pub async fn start(&self) -> Result<(), ServerError> { ... }
}

impl IServer for QueueServer {
    async fn close(&self) -> Result<(), ServerError> { ... }
}
```

**Initialization sequence**:
1. Parse configuration
2. Create Loki database
3. Create extent store (memory or file)
4. Create handlers
5. Create middleware
6. Create HTTP server
7. Bind to localhost:10001 (default queue port)

### QueueGCManager (14.22)
```rust
pub struct QueueGCManager {
    metadata_store: Arc<dyn IQueueMetadataStore>,
    extent_store: Arc<dyn IExtentStore>,
    gc_interval: i32,      // ms
    gc_max_items: i32,
    keep_secs: i32,
}

impl IGCManager for QueueGCManager {
    async fn start(&self) -> Result<(), GCError> { ... }
    async fn stop(&self) -> Result<(), GCError> { ... }
}
```

**GC algorithm** (same as blob):
1. Query referred extents from metadata store
2. Get all extents from extent store
3. Find unreferenced extents (not in referred set)
4. Delete extents older than `keep_secs` threshold
5. Mark for deletion; actually delete next cycle

### Utils (14.23-14.24)
```rust
// Queue-specific constants
pub const QUEUE_PORT: u16 = 10001;
pub const DEFAULT_QUEUE_DATABASE_PATH: &str = "./db/queue";
pub const DEFAULT_QUEUE_EXTENT_STORE_PATH: &str = "./extents/queue";

// Queue-specific utilities
pub fn validate_queue_name(name: &str) -> Result<(), ValidationError> { ... }
pub fn is_valid_message_id(id: &str) -> bool { ... }
pub fn is_valid_pop_receipt(receipt: &str) -> bool { ... }
```

### main.ts (14.21)
```rust
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    
    // Create configuration
    let config = QueueConfiguration::new(args)?;
    
    // Create logger
    let logger = create_logger(&config)?;
    
    // Create server
    let server = QueueServer::new(Arc::new(config))?;
    
    // Start server
    server.start().await?;
    
    Ok(())
}
```

## Dependencies

- Phase 1: IEnvironment, ConfigurationBase, ServerBase
- Phase 2: Loki persistence, extent store
- Phase 4: Logger, configuration
- Phase 14.1-14.2-3: Generated framework, errors
- Phase 14.7-14.8: Metadata store
- Phase 14.10-14.14: Handlers
- Phase 14.15: Middlewares

## Type Mappings

| TypeScript | Rust Equivalent | Notes |
|-----------|-----------------|-------|
| `IQueueEnvironment` | `pub trait IQueueEnvironment` | Interface implementation |
| `QueueConfiguration` | `pub struct QueueConfiguration` | Configuration holder |
| `QueueServer` | `pub struct QueueServer` | Server main entry point |
| `QueueGCManager` | `pub struct QueueGCManager` | Garbage collection manager |
| `number` | `i32`, `u16`, `i32` | Timeouts, ports, intervals |
| `string` | `String` | Paths, environment variables |
| `Error` | Custom error enum | Error handling |
| `argv` | `Vec<String>` | Command-line arguments |

## Special Handling / Fidelity Flags

### Critical Bootstrap Sequence
1. **Configuration parsing MUST come before logger creation**:
   - Some log output uses config values (e.g., log level)
   - Invalid config → early error before logger fully initialized

2. **Environment validation is LAZY**:
   - `QueueEnvironment` doesn't validate in constructor
   - Getters validate on access (allows missing env vars until needed)
   - Example: `queue_database_path()` checks for invalid characters only when called

3. **Persistence initialization order**:
   - Loki database must load before handlers (uses existing collections)
   - Extent store must initialize before handlers (extent store access path)
   - Handlers created after persistence fully initialized

4. **GC manager startup is async**:
   - Must start AFTER server is listening (avoid startup interference)
   - Runs in background loop; can be stopped independently
   - No await on GC in main (fire-and-forget)

### Comparison to Blob Bootstrap (Phase 12)
- **Identical pattern**: Same configuration, environment, server, GC patterns
- **Same port assignment**: Blob 10000, Queue 10001 (consecutive)
- **Same middleware order**: Identical request processing pipeline
- **Same persistence**: Both use Loki + extent store

### Configuration Validation Quirks
- **Mutual exclusivity check**: `inMemoryPersistence XOR location`
- **Path validation**: Must be writable; no existence check at startup
- **CLI args precedence**: Command-line args override environment variables
- **Lazy validation**: Errors deferred until value accessed (not in constructor)

## Change Propagation

**Middleware addition/removal:**
- Must update QueueRequestListenerFactory assembly order
- New middleware → must integrate into correct pipeline position

**Handler registration changes:**
- QueueRequestListenerFactory depends on handler instantiation
- New handler type → must register in factory

**Environment variable changes:**
- Any new queue-specific config → add to QueueEnvironment getter
- Name consistency (AZURITE_QUEUE_* prefix)

**Port changes:**
- Default port defined in QueueServer or constants
- Env var override available (AZURITE_QUEUE_PORT if supported)

## Rust Porting Notes

1. **Async main**: Use `#[tokio::main]` attribute for async runtime
2. **Arc<Mutex<T>>**: Use for shared, mutable persistence stores
3. **Environment variables**: Use `std::env::var()` with defaults
4. **Error propagation**: Use `?` operator for early returns
5. **Configuration validation**: Implement in setter or getter methods
6. **Logger initialization**: Set up before other components
7. **Graceful shutdown**: Handle SIGINT/SIGTERM for clean server stop
8. **Port binding**: Use `tokio::net::TcpListener` for HTTP server
9. **Background tasks**: Use `tokio::spawn()` for GC manager async loop
10. **Path handling**: Use `std::path::Path` for file system operations
11. **Default values**: Define as module-level constants
12. **CLI argument parsing**: Consider `clap` crate for cleaner argument handling

## Implementation Strategy

Propose 3-file porting units for Phase 14.16-14.24:
1. **Unit 1**: IQueueEnvironment + QueueEnvironment + QueueConfiguration (config/env setup)
2. **Unit 2**: QueueRequestListenerFactory + QueueServer (server assembly)
3. **Unit 3**: QueueGCManager + constants + utils + main (GC + entry point)
