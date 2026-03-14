# Porting Record — `src/blob/persistence/QueryInterpreter/QueryParser.ts`

## File info
- Source path: `src/blob/persistence/QueryInterpreter/QueryParser.ts`
- Source lines: `605`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/query_interpreter/query_parser.rs`
- Crate: `azurite-blob`
- Module: `persistence::query_interpreter::query_parser`
- Phase: `10.5`
- Status: `ported`

## Exported API
### Exported API
- `parseQuery(requestContext: Context, query: string, conditionsHeader?: string): IQueryNode`
- `ParserContext` tokenizer/helper class

### Internal parser types
- `enum ComparisonType { Equal, Greater, Less, NotEqual }`
- `interface ComparisonNode { key: string; existedComparison: ComparisonType[] }`
- `class QueryParser` recursive-descent parser with `visit*()` methods for query/expression/or/and/group/binary/value/key/string parsing

## Dependencies
- `StorageError` and `StorageErrorFactory` for two different error surfaces.
- `../generated/Context` for request IDs in parser errors.
- All query node classes under `QueryNodes/`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| recursive-descent parser class | hand-written parser struct | Port literally; the grammar is already encoded as mutually-recursive methods. |
| `comparisonNodes: Record<string, ComparisonNode>` | `HashMap<String, ComparisonNode>` | Tracks per-tag comparison constraints for `where` queries. |
| `ParserContext.tokenPosition` | `usize` cursor | Error messages report the current character position directly from this cursor. |

## Special handling
- `QueryParser.ts:42-60` documents a grammar with unary `not`, but `visitUnary()` at `225-228` never consumes `not`; the implementation currently ignores that grammar production entirely.
- `QueryParser.ts:79-140` applies range/duplicate-comparison validation only when `conditionHeader` is absent. Header-driven conditions skip both the uniqueness checks and the 10-unique-tag limit.
- `QueryParser.ts:183-195` and `272-278` only allow `or` and `<>` when parsing a conditional header, not the `where` query parameter.
- `QueryParser.ts:329-339` allows the special `@container` identifier only for the `where` parameter and rejects it for header conditions.
- `QueryParser.ts:311-315` only treats single quotes as value delimiters, while `visitKey()` at `428-439` allows double-quoted keys. The comment mentions both quote types for strings, but the implementation is asymmetric.
- `QueryParser.ts:377-418` uses a small state machine to support doubled quote escaping inside strings.
- `QueryParser.ts:586-605` emits two distinct error shapes: `StorageError(InvalidQueryParameterValue)` with character positions for `where`, or `StorageErrorFactory.getInvalidHeaderValue()` for headers.

## Change propagation notes
- If TS later implements unary `not`, update the parser, node set, and query-interpreter parity expectations together.
- Any grammar change must be mirrored alongside error-message expectations because tests will likely inspect those exact strings.
