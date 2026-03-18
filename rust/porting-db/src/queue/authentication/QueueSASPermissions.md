# Porting Record — `src/queue/authentication/QueueSASPermissions.ts`

## File info
- Source path: `src/queue/authentication/QueueSASPermissions.ts`
- Source lines: `6`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/authentication/queue_sas_permissions.rs`
- Crate: `azurite-queue`
- Module: `authentication::queue_sas_permissions`
- Status: `ported`

## Special handling
Queue SAS permission enum: Read, Add, Update, Process. Differs from blob (racwd).
