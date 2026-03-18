# Porting Record — `src/table/authentication/TableSharedKeyAuthenticator.ts`

## File info
- Source path: `src/table/authentication/TableSharedKeyAuthenticator.ts`
- Source lines: `356`
- Source type: `handwritten`
- Rust target: `azurite-table/src/authentication/table_shared_key_authenticator.rs`
- Crate: `azurite-table`
- Module: `authentication::table_shared_key_authenticator`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `string` | `String` | Account/queue names |
| `Buffer` | `Vec<u8>` | HMAC key material |
| `Promise<boolean | undefined>` | `Result<Option<bool>>` | Auth result |
| `IRequest` | `&dyn IRequest` | Request trait object |

## Special handling
SharedKey auth for table. Same HMAC-SHA256 pattern. Follows same pattern as blob equivalent. See `porting-db/src/blob/authentication/BlobSharedKeyAuthenticator.md` for detailed fidelity notes.
