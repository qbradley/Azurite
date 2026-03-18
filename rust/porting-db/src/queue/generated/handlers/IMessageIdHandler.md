# Porting Record — `src/queue/generated/handlers/IMessageIdHandler.ts`

## File info
- Source path: `src/queue/generated/handlers/IMessageIdHandler.ts`
- Source lines: `~50`
- Source type: `generated framework`
- Rust target: `azurite-queue/src/generated/handlers/i_message_id_handler.rs`
- Crate: `azurite-queue`
- Module: `generated::handlers::i_message_id_handler`
- Status: `ported`

## Special handling
Handler trait for /queue/messages/{messageId} operations (update, delete). Queue-specific handler interface — no direct blob equivalent.
