# Porting Record — `src/blob/generated/errors/UnsupportedRequestError.ts`

## File info
- Source path: `src/blob/generated/errors/UnsupportedRequestError.ts`
- Source lines: `11`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/errors/unsupported_request_error.rs`
- Crate: `azurite-blob`
- Module: `generated::errors::unsupported_request_error`
- Phase: `5.13`
- Status: `ported`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default class UnsupportedRequestError extends MiddlewareError {
  public constructor() {
    super(
      400,
      "Incoming URL doesn't match any of swagger defined request patterns."
    );
    this.name = "UnsupportedRequestError";
  }
}
```

## Dependencies
- Internal imports:
  - `./MiddlewareError` → `src/blob/generated/errors/MiddlewareError.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| constructor-only default error | zero-arg constructor / const factory | HTTP 400 when no operation matches dispatch. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- UnsupportedRequestError is part of the generated middleware error family; queue/table generated stacks will mirror the same pattern.
- These classes are intentionally tiny and message/status driven. Preserve exact status codes and default strings so downstream tests can diff regenerated TS output mechanically.

## Middleware chain ordering
- Constructed upstream, then consumed by `middleware/error.middleware.ts` before `end.middleware.ts` finalizes the response.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If autorest changes an error message or adds fields, update both the error class and `error.middleware.ts`, because middleware serialization is field-by-field.
