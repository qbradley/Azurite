# Porting Record — `src/table/authentication/OperationTableSASPermission.ts`

## File info
- Source path: `src/table/authentication/OperationTableSASPermission.ts`
- Source lines: `71`
- Source type: `handwritten`
- Rust target: `azurite-table/src/authentication/operation_table_sas_permission.rs`
- Crate: `azurite-table`
- Module: `authentication::operation_table_sas_permission`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `string` | `String` | Account/queue names |
| `Buffer` | `Vec<u8>` | HMAC key material |
| `Promise<boolean | undefined>` | `Result<Option<bool>>` | Auth result |
| `IRequest` | `&dyn IRequest` | Request trait object |

## Special handling
Maps table operations to table-level SAS permission letters `raud` (read/add/update/delete).
