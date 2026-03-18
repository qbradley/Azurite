# Porting Record — `src/table/authentication/TableSASAuthenticator.ts`

## File info
- Source path: `src/table/authentication/TableSASAuthenticator.ts`
- Source lines: `398`
- Source type: `handwritten`
- Rust target: `azurite-table/src/authentication/table_sas_authenticator.rs`
- Crate: `azurite-table`
- Module: `authentication::table_sas_authenticator`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `string` | `String` | Account/queue names |
| `Buffer` | `Vec<u8>` | HMAC key material |
| `Promise<boolean | undefined>` | `Result<Option<bool>>` | Auth result |
| `IRequest` | `&dyn IRequest` | Request trait object |

## Special handling
Table-level SAS token validator. Follows same pattern as blob equivalent. See `porting-db/src/blob/authentication/BlobSASAuthenticator.md` for detailed fidelity notes.
