# Porting Record — `src/blob/generated/errors/DeserializationError.ts`

## File info
- Source path: `src/blob/generated/errors/DeserializationError.ts`
- Source lines: `8`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/errors/deserialization_error.rs`
- Crate: `azurite-blob`
- Module: `generated::errors::deserialization_error`
- Phase: `5.13`
- Status: `ported`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default class DeserializationError extends MiddlewareError {
  public constructor(message: string) {
    super(400, message);
    this.name = "DeserializationError";
  }
}
```

## Dependencies
- Internal imports:
  - `./MiddlewareError` → `src/blob/generated/errors/MiddlewareError.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| constructor(message: string) | constructor(message: String) | HTTP 400 wraps deserializer failures. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- DeserializationError is part of the generated middleware error family; queue/table generated stacks will mirror the same pattern.
- These classes are intentionally tiny and message/status driven. Preserve exact status codes and default strings so downstream tests can diff regenerated TS output mechanically.

## Middleware chain ordering
- Constructed upstream, then consumed by `middleware/error.middleware.ts` before `end.middleware.ts` finalizes the response.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If autorest changes an error message or adds fields, update both the error class and `error.middleware.ts`, because middleware serialization is field-by-field.
