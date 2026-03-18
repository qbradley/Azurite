# Porting Record — `src/table/persistence/QueryInterpreter/QueryLexer.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryLexer.ts`
- Source lines: `195`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/query_lexer.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::query_lexer`
- Status: `ported`

## Special handling
Tokenizes OData $filter strings into tokens (operators, identifiers, literals, parentheses). Handles string literals with single-quote escaping, datetime/guid/binary literal prefixes.
