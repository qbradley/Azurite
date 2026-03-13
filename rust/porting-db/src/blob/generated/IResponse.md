# Porting Record — `src/blob/generated/IResponse.ts`

## File info
- Source path: `src/blob/generated/IResponse.ts`
- Source lines: `17`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/i_response.rs`
- Crate: `azurite-blob`
- Module: `generated::i_response`
- Phase: `5.2`
- Status: `analyzed`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default interface IResponse {
  setStatusCode(code: number): IResponse;
  getStatusCode(): number;
  setStatusMessage(message: string): IResponse;
  getStatusMessage(): string;
  setHeader(
    field: string,
    value?: string | string[] | undefined | number | boolean
  ): IResponse;
  getHeader(field: string): number | string | string[] | undefined;
  getHeaders(): OutgoingHttpHeaders;
  headersSent(): boolean;
  setContentType(value: string | undefined): IResponse;
  getBodyStream(): NodeJS.WritableStream;
}
```

## Dependencies
- Node built-ins:
  - `http` — imported as `{ OutgoingHttpHeaders }`

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `NodeJS.WritableStream` | `dyn AsyncWrite + Send` | Serializer and end middleware write directly to the response stream. |
| `OutgoingHttpHeaders` | `HeaderMap<String, HeaderValue>` with repeated-value support | Preserve header inspection for logging and middleware errors. |
| `string | string[] | number | boolean` header input | header setter helper that coerces numerics/bools to strings | Concrete coercion happens in `ExpressResponseAdapter`. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## API contract notes
- `headersSent()` gates `error.middleware.ts`; preserve this surface explicitly.
- `setContentType(value: string | undefined)` accepts `undefined` at the interface boundary even though the adapter only forwards strings.

## Middleware chain ordering
- Consumed by serializer, error, and end middleware; not itself a middleware stage.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If response wrappers gain trailer/body helpers in regenerated TS, add them to adapters and to middleware expectations together.
