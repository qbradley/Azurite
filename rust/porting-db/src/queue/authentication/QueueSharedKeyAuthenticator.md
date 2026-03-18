# Porting Record — `src/queue/authentication/QueueSharedKeyAuthenticator.ts`

## File info
- Source path: `src/queue/authentication/QueueSharedKeyAuthenticator.ts`
- Source lines: `356`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/authentication/queue_shared_key_authenticator.rs`
- Crate: `azurite-queue`
- Module: `authentication::queue_shared_key_authenticator`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `string` | `String` | Account/queue names |
| `Buffer` | `Vec<u8>` | HMAC key material |
| `Promise<boolean | undefined>` | `Result<Option<bool>>` | Auth result |
| `IRequest` | `&dyn IRequest` | Request trait object |

## Special handling
SharedKey auth for queue. Same HMAC-SHA256 algorithm as blob but queue-specific headers. Follows same pattern as blob equivalent. See `porting-db/src/blob/authentication/BlobSharedKeyAuthenticator.md` for detailed fidelity notes.
