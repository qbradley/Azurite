# Porting Record — `src/table/ITableEnvironment.ts`

## File info
- Source path: `src/table/ITableEnvironment.ts`
- Source lines: `29`
- Source type: `handwritten`
- Rust target: `azurite-table/src/i_table_environment.rs`
- Crate: `azurite-table`
- Module: `i_table_environment`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `interface ITableEnvironment` | `trait ITableEnvironment` | Environment config trait |

## Special handling
Table environment interface. Parallel to queue/blob environment traits. Defines table-specific config accessors (tableHost, tablePort).
