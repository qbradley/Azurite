# Porting Record — `src/table/authentication/TableSharedKeyLiteAuthenticator.ts`

## File info
- Source path: `src/table/authentication/TableSharedKeyLiteAuthenticator.ts`
- Source lines: `200`
- Source type: `handwritten`
- Rust target: `azurite-table/src/authentication/table_shared_key_lite_authenticator.rs`
- Crate: `azurite-table`
- Module: `authentication::table_shared_key_lite_authenticator`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `string` | `String` | Account/queue names |
| `Buffer` | `Vec<u8>` | HMAC key material |
| `Promise<boolean | undefined>` | `Result<Option<bool>>` | Auth result |
| `IRequest` | `&dyn IRequest` | Request trait object |

## Special handling
SharedKey Lite auth — table-specific, not present in blob or queue. Uses simplified canonical string with only path and Date header. Important for legacy Azure Storage SDK clients.
