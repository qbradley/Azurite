# Porting Record — `src/table/authentication/TableTokenAuthenticator.ts`

## File info
- Source path: `src/table/authentication/TableTokenAuthenticator.ts`
- Source lines: `253`
- Source type: `handwritten`
- Rust target: `azurite-table/src/authentication/table_token_authenticator.rs`
- Crate: `azurite-table`
- Module: `authentication::table_token_authenticator`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `string` | `String` | Account/queue names |
| `Buffer` | `Vec<u8>` | HMAC key material |
| `Promise<boolean | undefined>` | `Result<Option<bool>>` | Auth result |
| `IRequest` | `&dyn IRequest` | Request trait object |

## Special handling
Azure AD Bearer token validator for table. Follows same pattern as blob equivalent. See `porting-db/src/blob/authentication/BlobTokenAuthenticator.md` for detailed fidelity notes.
