# Porting Record — `src/queue/authentication/OperationAccountSASPermission.ts`

## File info
- Source path: `src/queue/authentication/OperationAccountSASPermission.ts`
- Source lines: `227`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/authentication/operation_account_sas_permission.rs`
- Crate: `azurite-queue`
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
Maps queue operations to account-level SAS permission letters (raup). Follows same pattern as blob equivalent. See `porting-db/src/blob/authentication/OperationAccountSASPermission.md` for detailed fidelity notes.
