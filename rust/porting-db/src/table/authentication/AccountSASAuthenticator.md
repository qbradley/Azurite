# Porting Record — `src/table/authentication/AccountSASAuthenticator.ts`

## File info
- Source path: `src/table/authentication/AccountSASAuthenticator.ts`
- Source lines: `315`
- Source type: `handwritten`
- Rust target: `azurite-table/src/authentication/account_sas_authenticator.rs`
- Crate: `azurite-table`
- Module: `authentication::account_sas_authenticator`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `string` | `String` | Account/queue names |
| `Buffer` | `Vec<u8>` | HMAC key material |
| `Promise<boolean | undefined>` | `Result<Option<bool>>` | Auth result |
| `IRequest` | `&dyn IRequest` | Request trait object |

## Special handling
Follows same pattern as blob equivalent. See `porting-db/src/blob/authentication/AccountSASAuthenticator.md` for detailed fidelity notes.
