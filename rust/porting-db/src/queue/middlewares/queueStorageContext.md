# Porting Record — `src/queue/middlewares/queueStorageContext.middleware.ts`

## File info
- Source path: `src/queue/middlewares/queueStorageContext.middleware.ts`
- Source lines: `120`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/middlewares/queue_storage_context.rs`
- Crate: `azurite-queue`
- Module: `middlewares::queue_storage_context`
- Status: `ported`

## Special handling
Extracts queue name, message ID, account from URL path into QueueStorageContext. Parallel to blob's blobStorageContext.middleware but with queue-specific path parsing (/accountname/queuename/messages/messageid).
