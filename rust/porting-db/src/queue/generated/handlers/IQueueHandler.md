# Porting Record — `src/queue/generated/handlers/IQueueHandler.ts`

## File info
- Source path: `src/queue/generated/handlers/IQueueHandler.ts`
- Source lines: `~50`
- Source type: `generated framework`
- Rust target: `azurite-queue/src/generated/handlers/i_queue_handler.rs`
- Crate: `azurite-queue`
- Module: `generated::handlers::i_queue_handler`
- Status: `ported`

## Special handling
Handler trait for /queue operations (create, delete, getProperties, setMetadata, getACL, setACL). Queue-specific handler interface — no direct blob equivalent.
