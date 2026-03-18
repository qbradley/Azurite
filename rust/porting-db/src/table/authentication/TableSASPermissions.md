# Porting Record — `src/table/authentication/TableSASPermissions.ts`

## File info
- Source path: `src/table/authentication/TableSASPermissions.ts`
- Source lines: `6`
- Source type: `handwritten`
- Rust target: `azurite-table/src/authentication/table_sas_permissions.rs`
- Crate: `azurite-table`
- Module: `authentication::table_sas_permissions`
- Status: `ported`

## Special handling
Table SAS permission enum: Read, Add, Update, Delete. Differs from queue (raup) and blob (racwd).
