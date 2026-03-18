# Porting Record — `src/queue/generated/handlers/IMessagesHandler.ts`

## File info
- Source path: `src/queue/generated/handlers/IMessagesHandler.ts`
- Source lines: `~50`
- Source type: `generated framework`
- Rust target: `azurite-queue/src/generated/handlers/i_messages_handler.rs`
- Crate: `azurite-queue`
- Module: `generated::handlers::i_messages_handler`
- Status: `ported`

## Special handling
Handler trait for /queue/messages operations (enqueue, dequeue, peek, clear). Queue-specific handler interface — no direct blob equivalent.
