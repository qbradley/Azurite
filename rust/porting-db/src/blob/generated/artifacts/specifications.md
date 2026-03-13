# Porting Record — `src/blob/generated/artifacts/specifications.ts`

## File info
- Source path: `src/blob/generated/artifacts/specifications.ts`
- Source lines: `2825`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/artifacts/specifications.rs`
- Crate: `azurite-blob`
- Module: `generated::artifacts::specifications`
- Phase: `5.12`
- Status: `analyzed`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default Specifications;
```

## Dependencies
- External packages:
  - `@azure/ms-rest-js` — imported as `* as msRest`
- Internal imports:
  - `./mappers` → `src/blob/generated/artifacts/mappers.ts` — Phase 5 — analyzed in this pass
  - `./operation` → `src/blob/generated/artifacts/operation.ts` — Phase 5 — analyzed in this pass
  - `./parameters` → `src/blob/generated/artifacts/parameters.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `msRest.OperationSpec` consts | `static OperationSpec` descriptors | Preserve HTTP method, path, parameter lists, request body mapper, response map, and XML flag exactly. |
| `new msRest.Serializer(Mappers, true)` | shared serializer instance / singleton mapper registry | The same serializer instance is attached to every operation spec. |
| `default Specifications` numeric map | `static array / HashMap<Operation, OperationSpec>` | Indexed by the `Operation` enum numeric value. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- Defines 72 operation-spec constants and assigns 72 of them into the exported `Specifications` map.
- Shared serializer initialization is `new msRest.Serializer(Mappers, true)`; keep the mapper registry and XML-aware behavior coupled.
- Current generated blob specs all set `isXML: true` (72 occurrences) even when response/request bodies are streams or JSON-like wrappers; that flag still influences serializer/deserializer body handling.
- 12 operations declare an explicit request body section.

## Middleware chain ordering
- Dispatch uses these specs to select the operation.
- Deserializer uses the chosen spec to decode query/header/body parameters.
- Serializer uses the chosen spec to map `handlerResponses` into status/header/body output.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- A regenerated spec diff must be propagated to dispatch logic, serializer expectations, handler signatures, and response wrapper types as one unit.
- If default error body mappers or status-code maps change, update `error.middleware.ts` behavior review because handler vs middleware errors may need new parity tests.
