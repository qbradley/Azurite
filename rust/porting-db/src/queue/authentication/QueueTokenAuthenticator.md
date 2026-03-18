# Porting Record — `src/queue/authentication/QueueTokenAuthenticator.ts`

## File info
- Source path: `src/queue/authentication/QueueTokenAuthenticator.ts`
- Source lines: `253`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/authentication/queue_token_authenticator.rs`
- Crate: `azurite-queue`
- Module: `authentication::queue_token_authenticator`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `string` | `String` | Account/queue names |
| `Buffer` | `Vec<u8>` | HMAC key material |
| `Promise<boolean | undefined>` | `Result<Option<bool>>` | Auth result |
| `IRequest` | `&dyn IRequest` | Request trait object |

## Special handling
Azure AD Bearer token validator for queue. Follows same pattern as blob equivalent. See `porting-db/src/blob/authentication/BlobTokenAuthenticator.md` for detailed fidelity notes.
