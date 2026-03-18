# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/BigNumberNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/BigNumberNode.ts`
- Source lines: `~30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/big_number_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::big_number_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `impl IQueryNode` | AST node impl |

## Special handling
Large number literal for Int64 comparisons. TS uses BigNumber library; Rust uses i64.
