# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/BinaryOperatorNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/BinaryOperatorNode.ts`
- Source lines: `~30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/binary_operator_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::binary_operator_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `impl IQueryNode` | AST node impl |

## Special handling
Abstract base for binary comparison operators. Children: left (IdentifierNode) + right (value node).
