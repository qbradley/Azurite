# Porting Record — `src/table/authentication/ITableSASSignatureValues.ts`

## File info
- Source path: `src/table/authentication/ITableSASSignatureValues.ts`
- Source lines: `155`
- Source type: `handwritten`
- Rust target: `azurite-table/src/authentication/i_table_sas_signature_values.rs`
- Crate: `azurite-table`
- Module: `authentication::i_table_sas_signature_values`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `string` | `String` | Account/queue names |
| `Buffer` | `Vec<u8>` | HMAC key material |
| `Promise<boolean | undefined>` | `Result<Option<bool>>` | Auth result |
| `IRequest` | `&dyn IRequest` | Request trait object |

## Special handling
SAS signature values for table service. Canonical resource: `/tableservices/accountname/tablename`. Table SAS uses `raud` permissions (read/add/update/delete).
