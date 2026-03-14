# Porting Record — `src/blob/persistence/QueryInterpreter/QueryNodes/ConstantNode.ts`

## File info
- Source path: `src/blob/persistence/QueryInterpreter/QueryNodes/ConstantNode.ts`
- Source lines: `19`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/query_interpreter/query_nodes/constant_node.rs`
- Crate: `azurite-blob`
- Module: `persistence::query_interpreter::query_nodes::constant_node`
- Phase: `10.4`
- Status: `ported`

## Exported API
### Default class `ConstantNode`
- Implements `IQueryNode`.
- Constructor: `new ConstantNode(value: string)`
- Members: `name`, `evaluate(context)`, `toString()`

## Dependencies
- `../IQueryContext`.
- `./IQueryNode`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| string literal node | leaf AST variant carrying a `String` | Stores the parsed constant value exactly as the parser produced it. |
| `evaluate(_context)` | returns `Vec<TagContent>` with value-only witness | Constants ignore runtime context and contribute only a value. |

## Special handling
- `ConstantNode.ts:11-15` returns `[{ value: this.value }]` with no key field set.
- `ConstantNode.ts:17-18` uses `JSON.stringify(this.value)` for `toString()`, so embedded quotes/escapes follow JSON rules rather than raw single-quoted query syntax.

## Change propagation notes
- If TS starts preserving original quote style in `toString()`, update this record and any Rust debug formatting.
- Comparison-node behavior depends on constants lacking `key`; do not invent one in Rust.
