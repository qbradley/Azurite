# Porting Record — `src/queue/authentication/QueueSASAuthenticator.ts`

## File info
- Source path: `src/queue/authentication/QueueSASAuthenticator.ts`
- Source lines: `398`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/authentication/queue_sas_authenticator.rs`
- Crate: `azurite-queue`
- Module: `authentication::queue_sas_authenticator`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `string` | `String` | Account/queue names |
| `Buffer` | `Vec<u8>` | HMAC key material |
| `Promise<boolean | undefined>` | `Result<Option<bool>>` | Auth result |
| `IRequest` | `&dyn IRequest` | Request trait object |

## Special handling
Queue-level SAS token validator. Validates signature against queue-scoped canonical resource. Follows same pattern as blob equivalent. See `porting-db/src/blob/authentication/BlobSASAuthenticator.md` for detailed fidelity notes.
