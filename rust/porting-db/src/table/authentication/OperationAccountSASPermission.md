# Porting Record — `src/table/authentication/OperationAccountSASPermission.ts`

## File info
- Source path: `src/table/authentication/OperationAccountSASPermission.ts`
- Source lines: `227`
- Source type: `handwritten`
- Rust target: `azurite-table/src/authentication/operation_account_sas_permission.rs`
- Crate: `azurite-table`
- Module: `authentication::operation_account_sas_permission`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `string` | `String` | Account/queue names |
| `Buffer` | `Vec<u8>` | HMAC key material |
| `Promise<boolean | undefined>` | `Result<Option<bool>>` | Auth result |
| `IRequest` | `&dyn IRequest` | Request trait object |

## Special handling
Maps table operations to account-level SAS permission letters. Follows same pattern as blob equivalent. See `porting-db/src/blob/authentication/OperationAccountSASPermission.md` for detailed fidelity notes.
