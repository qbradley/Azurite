# Porting Record — `src/blob/generated/errors/MiddlewareError.ts`

## File info
- Source path: `src/blob/generated/errors/MiddlewareError.ts`
- Source lines: `29`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/errors/middleware_error.rs`
- Crate: `azurite-blob`
- Module: `generated::errors::middleware_error`
- Phase: `5.13`
- Status: `ported`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default class MiddlewareError extends Error {
  /**
   * Creates an instance of MiddlewareError.
   *
   * @param {number} statusCode HTTP response status code
   * @param {string} message Error message
   * @param {string} [statusMessage] HTTP response status message
   * @param {OutgoingHttpHeaders} [headers] HTTP response headers
   * @param {string} [body] HTTP response body
   * @param {string} [contentType] HTTP contentType
   * @memberof MiddlewareError
   */
  constructor(
    public readonly statusCode: number,
    public readonly message: string,
    public readonly statusMessage?: string,
    public readonly headers?: OutgoingHttpHeaders,
    public readonly body?: string,
    public readonly contentType?: string
  ) {
    super(message);
    // https://stackoverflow.com/questions/31626231/custom-error-class-in-typescript
    Object.setPrototypeOf(this, MiddlewareError.prototype);

    this.name = "MiddlewareError";
  }
}
```

## Dependencies
- Node built-ins:
  - `http` — imported as `{ OutgoingHttpHeaders }`

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `class MiddlewareError extends Error` | `struct MiddlewareError` implementing `std::error::Error` | Carry HTTP response metadata fields explicitly. |
| `OutgoingHttpHeaders` | `HeaderMap` | Optional headers are serialized by `error.middleware.ts`. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- MiddlewareError is part of the generated middleware error family; queue/table generated stacks will mirror the same pattern.
- These classes are intentionally tiny and message/status driven. Preserve exact status codes and default strings so downstream tests can diff regenerated TS output mechanically.

## Middleware chain ordering
- Constructed upstream, then consumed by `middleware/error.middleware.ts` before `end.middleware.ts` finalizes the response.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If autorest changes an error message or adds fields, update both the error class and `error.middleware.ts`, because middleware serialization is field-by-field.
