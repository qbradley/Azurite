# Porting Record — `src/queue/authentication/OperationQueueSASPermission.ts`

## File info
- Source path: `src/queue/authentication/OperationQueueSASPermission.ts`
- Source lines: `71`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/authentication/operation_queue_sas_permission.rs`
- Crate: `azurite-queue`
- Module: `authentication::operation_queue_sas_permission`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `string` | `String` | Account/queue names |
| `Buffer` | `Vec<u8>` | HMAC key material |
| `Promise<boolean | undefined>` | `Result<Option<bool>>` | Auth result |
| `IRequest` | `&dyn IRequest` | Request trait object |

## Special handling
Maps queue operations to queue-level SAS permission letters. Queue uses `raup` (read/add/update/process) vs blob `racwd`.
