# Porting Record — `src/blob/persistence/QueryInterpreter/QueryInterpreter.ts`

## File info
- Source path: `src/blob/persistence/QueryInterpreter/QueryInterpreter.ts`
- Source lines: `82`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/query_interpreter/query_interpreter.rs`
- Crate: `azurite-blob`
- Module: `persistence::query_interpreter::query_interpreter`
- Phase: `10.6`
- Status: `ported`

## Exported API
### Exported functions
- `executeQuery(context: FilterBlobModel, queryTree: IQueryNode): TagContent[]`
- `generateQueryBlobWithTagsWhereFunction(requestContext, query, conditionHeader?): (entity) => TagContent[]`

## Dependencies
- `StorageError` and `StorageErrorFactory` for post-parse validation errors.
- `BlobTags` from generated models, `Context`, and `FilterBlobModel`.
- `BinaryOperatorNode`, `ExpressionNode`, `IQueryNode`, and `parseQuery()`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `FilterBlobModel.tags` string-or-object union | explicit enum/adapter over raw JSON vs typed tags | TS accepts both stored JSON strings and already-materialized blob-tag objects. |
| `TagContent[]` return | `Vec<TagContent>` | The interpreter preserves the witness-array contract from query nodes. |
| lambda filter generator | boxed closure or helper fn | Used by `LokiBlobMetadataStore.filterBlobs()` to screen docs by tags. |

## Special handling
- `QueryInterpreter.ts:11-28` parses `context.tags` if it is a string, otherwise uses it as-is, then injects `@container = context.containerName` before evaluating the AST.
- `QueryInterpreter.ts:30-40` counts identifier references by recursing only through `BinaryOperatorNode` and `ExpressionNode`; leaves and constants count as zero.
- `QueryInterpreter.ts:48-52` returns a closure that always yields `[]` when `query === undefined`.
- `QueryInterpreter.ts:60-80` rejects constant-only query trees after parsing. For `where` it throws `InvalidQueryParameterValue` at character position 1; for headers it converts the failure into `InvalidHeaderValue`.

## Change propagation notes
- If TS ever changes how blob tags are persisted (string vs object), update this file, `filterBlobs()`, and tag utility helpers together.
- Keep the `@container` injection behavior explicit; it is part of the observable tag-query surface.
